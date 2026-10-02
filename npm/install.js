#!/usr/bin/env node
"use strict";

const https = require("https");
const http = require("http");
const tls = require("tls");
const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");
const { spawnSync } = require("child_process");

const MAX_REDIRECTS = 5;
const SOCKET_TIMEOUT_MS = 30000;

if (process.env.ANESIS_SKIP_INSTALL) {
	console.log(
		"anesis: ANESIS_SKIP_INSTALL is set, skipping binary download. " +
			"Place a binary at npm/bin/anesis (or anesis.exe on Windows) yourself.",
	);
	process.exit(0);
}

const PLATFORM_MAP = {
	"linux-x64": { name: "linux-x86_64", ext: "tar.gz", binary: "anesis" },
	"linux-arm64": { name: "linux-aarch64", ext: "tar.gz", binary: "anesis" },
	"darwin-arm64": { name: "macos-aarch64", ext: "tar.gz", binary: "anesis" },
	"darwin-x64": { name: "macos-x86_64", ext: "tar.gz", binary: "anesis" },
	"win32-x64": { name: "windows-x86_64", ext: "zip", binary: "anesis.exe" },
};

const key = `${process.platform}-${process.arch}`;
const platform = PLATFORM_MAP[key];
if (!platform) {
	console.error(
		`anesis: unsupported platform "${key}". ` +
			`Supported via npm: linux-x64, linux-arm64, darwin-arm64, darwin-x64, win32-x64. ` +
			`Install manually from https://github.com/anesis-dev/anesis-cli/releases`,
	);
	process.exit(1);
}

const pkg = JSON.parse(
	fs.readFileSync(path.join(__dirname, "package.json"), "utf8"),
);
const version = pkg.binaryVersion || pkg.version;

const binDir = path.join(__dirname, "bin");
const dest = path.join(binDir, platform.binary);
const versionFile = path.join(binDir, ".version");

if (
	fs.existsSync(dest) &&
	fs.existsSync(versionFile) &&
	fs.readFileSync(versionFile, "utf8").trim() === version
) {
	process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

const assetName = `anesis-${platform.name}.${platform.ext}`;
const releaseBaseUrl = `https://github.com/anesis-dev/anesis-cli/releases/download/v${version}`;
const url = `${releaseBaseUrl}/${assetName}`;
const sumsUrl = `${releaseBaseUrl}/SHA256SUMS`;

console.log(`anesis: downloading ${url}`);

function getProxyUrl() {
	const v =
		process.env.HTTPS_PROXY ||
		process.env.https_proxy ||
		process.env.HTTP_PROXY ||
		process.env.http_proxy ||
		process.env.npm_config_https_proxy ||
		process.env.npm_config_proxy;
	return v ? new URL(v) : null;
}

function tunnelConnect(proxyUrl, targetHost, targetPort) {
	return new Promise((resolve, reject) => {
		const headers = { Host: `${targetHost}:${targetPort}` };
		if (proxyUrl.username) {
			const auth = `${decodeURIComponent(proxyUrl.username)}:${decodeURIComponent(proxyUrl.password || "")}`;
			headers["Proxy-Authorization"] = `Basic ${Buffer.from(auth).toString("base64")}`;
		}
		const req = http.request({
			host: proxyUrl.hostname,
			port: proxyUrl.port || 80,
			method: "CONNECT",
			path: `${targetHost}:${targetPort}`,
			headers,
			timeout: SOCKET_TIMEOUT_MS,
		});
		req.on("connect", (res, socket) => {
			if (res.statusCode !== 200) {
				socket.destroy();
				reject(
					new Error(
						`proxy CONNECT to ${targetHost}:${targetPort} failed: HTTP ${res.statusCode}`,
					),
				);
				return;
			}
			resolve(socket);
		});
		req.on("timeout", () =>
			req.destroy(
				new Error(`proxy CONNECT to ${targetHost}:${targetPort} timed out`),
			),
		);
		req.on("error", reject);
		req.end();
	});
}

class ProxyAgent extends https.Agent {
	constructor(proxyUrl) {
		super();
		this.proxyUrl = proxyUrl;
	}
	createConnection(options, callback) {
		tunnelConnect(this.proxyUrl, options.host, options.port || 443)
			.then((socket) => {
				const tlsSocket = tls.connect({
					socket,
					servername: options.servername || options.host,
				});
				tlsSocket.once("secureConnect", () => callback(null, tlsSocket));
				tlsSocket.once("error", callback);
			})
			.catch((err) => callback(err));
	}
}

function download(url, cb, redirectCount) {
	redirectCount = redirectCount || 0;

	let settled = false;
	const finish = (err, buf) => {
		if (settled) return;
		settled = true;
		cb(err, buf);
	};

	if (redirectCount > MAX_REDIRECTS) {
		finish(new Error(`too many redirects downloading ${url}`));
		return;
	}

	const target = new URL(url);
	const proxyUrl = getProxyUrl();
	const options = {
		headers: { "User-Agent": "anesis-npm-installer" },
		timeout: SOCKET_TIMEOUT_MS,
	};
	if (proxyUrl) {
		options.agent = new ProxyAgent(proxyUrl);
	}

	const req = https.get(target, options, (res) => {
		if (
			res.statusCode >= 301 &&
			res.statusCode <= 308 &&
			res.headers.location
		) {
			res.resume();
			download(
				new URL(res.headers.location, target).toString(),
				finish,
				redirectCount + 1,
			);
			return;
		}
		if (res.statusCode !== 200) {
			res.resume();
			finish(new Error(`HTTP ${res.statusCode} downloading ${url}`));
			return;
		}
		const chunks = [];
		res.on("data", (c) => chunks.push(c));
		res.on("end", () => finish(null, Buffer.concat(chunks)));
		res.on("error", finish);
	});
	req.on("timeout", () =>
		req.destroy(new Error(`request to ${url} timed out`)),
	);
	req.on("error", finish);
}

function extractTarGz(buf, destPath) {
	const inflated = zlib.gunzipSync(buf);
	let offset = 0;
	while (offset + 512 <= inflated.length) {
		const header = inflated.slice(offset, offset + 512);
		const name = header.slice(0, 100).toString("utf8").replace(/\0.*$/, "");
		if (!name) break;
		const sizeStr = header
			.slice(124, 136)
			.toString("utf8")
			.replace(/\0.*$/, "")
			.trim();
		const size = parseInt(sizeStr, 8) || 0;
		const typeFlag = header[156];
		offset += 512;
		if (typeFlag === 0x30 || typeFlag === 0) {
			if (path.basename(name) === "anesis") {
				fs.writeFileSync(destPath, inflated.slice(offset, offset + size), {
					mode: 0o755,
				});
				console.log(`anesis: installed to ${destPath}`);
				return;
			}
		}
		offset += Math.ceil(size / 512) * 512;
	}
	throw new Error("anesis binary not found in archive");
}

function extractZip(buf, destPath) {
	let eocdOffset = -1;
	for (let i = buf.length - 22; i >= 0; i--) {
		if (buf.readUInt32LE(i) === 0x06054b50) {
			eocdOffset = i;
			break;
		}
	}
	if (eocdOffset === -1) throw new Error("Invalid ZIP: EOCD not found");

	const cdEntries = buf.readUInt16LE(eocdOffset + 8);
	const cdOffset = buf.readUInt32LE(eocdOffset + 16);

	let pos = cdOffset;
	for (let i = 0; i < cdEntries; i++) {
		if (buf.readUInt32LE(pos) !== 0x02014b50)
			throw new Error("Invalid ZIP central directory");
		const method = buf.readUInt16LE(pos + 10);
		const compressedSz = buf.readUInt32LE(pos + 20);
		const fileNameLen = buf.readUInt16LE(pos + 28);
		const extraLen = buf.readUInt16LE(pos + 30);
		const commentLen = buf.readUInt16LE(pos + 32);
		const localOffset = buf.readUInt32LE(pos + 42);
		const fileName = buf
			.slice(pos + 46, pos + 46 + fileNameLen)
			.toString("utf8");
		pos += 46 + fileNameLen + extraLen + commentLen;

		if (path.basename(fileName) === "anesis.exe") {
			if (buf.readUInt32LE(localOffset) !== 0x04034b50)
				throw new Error("Invalid ZIP local header");
			const localFileNameLen = buf.readUInt16LE(localOffset + 26);
			const localExtraLen = buf.readUInt16LE(localOffset + 28);
			const dataOffset = localOffset + 30 + localFileNameLen + localExtraLen;
			const compressed = buf.slice(dataOffset, dataOffset + compressedSz);

			const data = method === 0 ? compressed : zlib.inflateRawSync(compressed);
			fs.writeFileSync(destPath, data);
			console.log(`anesis: installed to ${destPath}`);
			return;
		}
	}
	throw new Error("anesis.exe not found in ZIP archive");
}

function verifyChecksum(buf, sumsBuf) {
	const line = sumsBuf
		.toString("utf8")
		.split("\n")
		.map((l) => l.trim())
		.find((l) => l.split(/\s+/)[1] === assetName);
	if (!line) {
		throw new Error(`no checksum for ${assetName} in SHA256SUMS`);
	}
	const expected = line.split(/\s+/)[0].toLowerCase();
	const actual = crypto.createHash("sha256").update(buf).digest("hex");
	if (expected !== actual) {
		throw new Error(
			`checksum mismatch for ${assetName}: expected ${expected}, got ${actual}`,
		);
	}
	console.log("anesis: checksum verified");
}

download(url, (err, buf) => {
	if (err) {
		console.error(`anesis: download failed: ${err.message}`);
		process.exit(1);
	}
	download(sumsUrl, (sumsErr, sumsBuf) => {
		if (sumsErr) {
			console.error(`anesis: SHA256SUMS download failed: ${sumsErr.message}`);
			process.exit(1);
		}
		try {
			verifyChecksum(buf, sumsBuf);
		} catch (e) {
			console.error(`anesis: ${e.message}`);
			process.exit(1);
		}
		try {
			if (platform.ext === "tar.gz") {
				extractTarGz(buf, dest);
			} else {
				extractZip(buf, dest);
			}
			fs.writeFileSync(versionFile, version);
			installCompletions(dest);
		} catch (e) {
			console.error(`anesis: extraction failed: ${e.message}`);
			process.exit(1);
		}
	});
});

function installCompletions(binaryPath) {
	const shell =
		process.platform === "win32"
			? "powershell"
			: path.basename(process.env.SHELL || "");
	if (!shell || !["bash", "zsh", "fish", "powershell"].includes(shell)) return;
	try {
		spawnSync(binaryPath, ["completions", shell], { stdio: "inherit" });
	} catch (_) {
	}
}
