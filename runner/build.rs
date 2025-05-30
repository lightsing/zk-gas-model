use sp1_build::{BuildArgs, build_program_with_args};

fn main() {
    build_program_with_args(
        "../guest",
        BuildArgs {
            output_directory: Some("elf".to_string()),
            ..Default::default()
        },
    );
}
