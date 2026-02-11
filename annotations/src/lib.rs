/// Embedded Java annotation files for use in the jbindgen CLI.
pub const ANNOTATION_FILES: &[(&str, &str)] = &[
    (
        "RustName.java",
        include_str!("main/java/io/github/jni_rs/jbindgen/RustName.java"),
    ),
    (
        "RustPrimitive.java",
        include_str!("main/java/io/github/jni_rs/jbindgen/RustPrimitive.java"),
    ),
    (
        "RustSkip.java",
        include_str!("main/java/io/github/jni_rs/jbindgen/RustSkip.java"),
    ),
    (
        "package-info.java",
        include_str!("main/java/io/github/jni_rs/jbindgen/package-info.java"),
    ),
];
