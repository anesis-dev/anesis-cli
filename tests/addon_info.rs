use anesis::addons::info::input_line_for_tests;
use anesis::addons::manifest::{
  AppendStep, CopyStep, CreateStep, DeleteStep, IfExists, IfNotFound, InjectStep, InputDef,
  InputType, MoveStep, PackagesStep, RenameStep, ReplaceStep, RunStep, Step, Target,
};
use anesis::addons::runner::step_label_for_tests;

fn input(
  input_type: InputType,
  required: bool,
  default: Option<&str>,
  options: &[&str],
) -> InputDef {
  InputDef {
    name: "db_engine".to_string(),
    input_type,
    description: String::new(),
    default: default.map(str::to_string),
    required,
    options: options.iter().map(|s| s.to_string()).collect(),
  }
}

#[test]
fn input_line_names_the_input_and_its_type() {
  let line = input_line_for_tests(&input(InputType::Text, false, None, &[]));
  assert!(line.contains("db_engine"));
  assert!(line.contains("text"));
}

#[test]
fn input_line_shows_boolean_and_select_types() {
  assert!(input_line_for_tests(&input(InputType::Boolean, false, None, &[])).contains("boolean"));
  assert!(input_line_for_tests(&input(InputType::Select, false, None, &[])).contains("select"));
}

#[test]
fn input_line_marks_required() {
  let line = input_line_for_tests(&input(InputType::Text, true, None, &[]));
  assert!(line.contains("required"));
}

#[test]
fn input_line_shows_the_default() {
  let line = input_line_for_tests(&input(InputType::Text, false, Some("postgres"), &[]));
  assert!(line.contains("default: postgres"));
}

#[test]
fn input_line_shows_options() {
  let line = input_line_for_tests(&input(
    InputType::Select,
    false,
    None,
    &["postgres", "mysql"],
  ));
  assert!(line.contains("postgres/mysql"));
}

#[test]
fn input_line_with_no_extras_has_no_parenthetical() {
  let line = input_line_for_tests(&input(InputType::Text, false, None, &[]));
  assert!(!line.contains('('));
}

#[test]
fn input_line_combines_required_default_and_options() {
  let line = input_line_for_tests(&input(
    InputType::Select,
    true,
    Some("postgres"),
    &["postgres", "mysql"],
  ));
  assert!(line.contains("required"));
  assert!(line.contains("default: postgres"));
  assert!(line.contains("options: postgres/mysql"));
}

#[test]
fn step_label_covers_every_step_variant() {
  let cases: Vec<(Step, &str)> = vec![
    (
      Step::Copy(CopyStep {
        src: "a".into(),
        dest: "b".into(),
        if_exists: IfExists::Overwrite,
        render: false,
      }),
      "copy 'a' → 'b'",
    ),
    (
      Step::Create(CreateStep {
        path: "f.txt".into(),
        content: String::new(),
        if_exists: IfExists::Overwrite,
      }),
      "create 'f.txt'",
    ),
    (
      Step::Inject(InjectStep {
        target: Target::File {
          file: "f.txt".into(),
        },
        content: String::new(),
        after: None,
        before: None,
        if_not_found: IfNotFound::Error,
      }),
      "inject into 'f.txt'",
    ),
    (
      Step::Replace(ReplaceStep {
        target: Target::Glob {
          glob: "*.txt".into(),
        },
        find: String::new(),
        replace: String::new(),
        if_not_found: IfNotFound::Error,
      }),
      "replace in '*.txt'",
    ),
    (
      Step::Append(AppendStep {
        target: Target::File {
          file: "f.txt".into(),
        },
        content: String::new(),
      }),
      "append to 'f.txt'",
    ),
    (
      Step::Delete(DeleteStep {
        target: Target::File {
          file: "f.txt".into(),
        },
      }),
      "delete 'f.txt'",
    ),
    (
      Step::Rename(RenameStep {
        from: "a".into(),
        to: "b".into(),
      }),
      "rename 'a' → 'b'",
    ),
    (
      Step::Move(MoveStep {
        from: "a".into(),
        to: "b".into(),
      }),
      "move 'a' → 'b'",
    ),
    (
      Step::Packages(PackagesStep {
        dependencies: vec!["left-pad".into()],
        dev_dependencies: vec!["jest".into(), "eslint".into()],
      }),
      "install 3 package(s)",
    ),
    (
      Step::Run(RunStep {
        command: "echo hi".into(),
        description: String::new(),
      }),
      "run 'echo hi'",
    ),
  ];

  for (step, expected) in cases {
    assert_eq!(step_label_for_tests(&step), expected);
  }
}

#[test]
fn step_label_resolves_a_glob_target_the_same_as_a_file_target() {
  let step = Step::Delete(DeleteStep {
    target: Target::Glob {
      glob: "dist/*.tmp".into(),
    },
  });
  assert_eq!(step_label_for_tests(&step), "delete 'dist/*.tmp'");
}
