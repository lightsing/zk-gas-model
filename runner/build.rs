use sp1_build::{BuildArgs, build_program_with_args};

fn main() {
    build_program_with_args(
        "../guest",
        BuildArgs {
            features: vec!["baseline".to_string()],
            output_directory: Some("elf/baseline".to_string()),
            ..Default::default()
        },
    );

    build_program_with_args(
        "../guest",
        BuildArgs {
            output_directory: Some("elf/exec".to_string()),
            ..Default::default()
        },
    );
}
