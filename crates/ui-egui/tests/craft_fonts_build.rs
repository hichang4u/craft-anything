//! The build script's craft-fonts decision (`build.rs`), tested as the script computes it: from the
//! two environment variables and what's on disk. A build told the fonts are required
//! (`CRAFT_FONTS_REQUIRED=1`, as releases and CI are) once succeeded with none embedded when
//! `CRAFT_FONTS_DIR` was missing, and every script but Latin showed as boxes.

#[allow(dead_code)]
#[path = "../build.rs"]
mod build;

use build::plan;
use std::ffi::OsString;
use std::path::Path;

const EMPTY_TABLE: &str = "pub static CRAFT_FONTS: &[CraftFont] = &[\n];\n";

fn var(v: &str) -> Option<OsString> {
    Some(v.into())
}

/// A craft-fonts checkout with two fonts, in the manifest's own format.
fn checkout(dir: &Path) {
    let fonts = dir.join("fonts");
    std::fs::create_dir_all(fonts.join("nanum-gothic")).unwrap();
    std::fs::create_dir_all(fonts.join("biz-ud-pgothic")).unwrap();
    std::fs::write(fonts.join("nanum-gothic/NanumGothic-Regular.ttf"), b"korean").unwrap();
    std::fs::write(fonts.join("biz-ud-pgothic/BIZUDPGothic-Regular.ttf"), b"japanese").unwrap();
    let manifest = "# family | style | file | scripts | licence | licence file | sha256 | source\n\n\
        NanumGothic | Regular | fonts/nanum-gothic/NanumGothic-Regular.ttf | Kore,Latn | OFL-1.1 | fonts/nanum-gothic/OFL.txt | 00 | \n\
        BIZ UDPGothic | Regular | fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf | Jpan | OFL-1.1 | fonts/biz-ud-pgothic/OFL.txt | 00 | \n";
    std::fs::write(fonts.join("manifest.txt"), manifest).unwrap();
}

#[test]
fn required_fonts_without_a_folder_fail_the_build() {
    for dir in [None, var("")] {
        for required in ["1", "true"] {
            let p = plan(dir.clone(), var(required), false);
            let error = p.error.expect("a build that requires the fonts can't go on without their folder");
            assert!(error.contains("CRAFT_FONTS_REQUIRED") && error.contains("CRAFT_FONTS_DIR"), "{error}");
            assert!(!error.contains('\n'), "one line, as Cargo reads it");
            assert_eq!((p.warning, p.source.as_str(), p.inputs.len()), (None, EMPTY_TABLE, 0));
        }
    }
}

#[test]
fn a_bad_folder_fails_the_build_only_when_the_fonts_are_required() {
    let dir = tempfile::tempdir().unwrap();
    let nothing_there = dir.path().join("nothing-there");
    let no_manifest = dir.path().join("no-manifest");
    std::fs::create_dir(&no_manifest).unwrap();
    let no_fonts = dir.path().join("no-fonts");
    std::fs::create_dir_all(no_fonts.join("fonts")).unwrap();
    std::fs::write(no_fonts.join("fonts/manifest.txt"), "# nothing listed\n\n").unwrap();
    let missing_file = dir.path().join("missing-file");
    checkout(&missing_file);
    std::fs::remove_file(missing_file.join("fonts/nanum-gothic/NanumGothic-Regular.ttf")).unwrap();
    let garbled = dir.path().join("garbled");
    std::fs::create_dir_all(garbled.join("fonts")).unwrap();
    std::fs::write(garbled.join("fonts/manifest.txt"), "NanumGothic, Regular\n").unwrap();
    for (bad, says) in [
        (&nothing_there, "manifest.txt"),
        (&no_manifest, "manifest.txt"),
        (&no_fonts, "lists no fonts"),
        (&missing_file, "NanumGothic-Regular.ttf"),
        (&garbled, "malformed"),
    ] {
        let required = plan(Some(bad.into()), var("1"), false);
        let error = required.error.unwrap_or_else(|| panic!("{} must fail a build that requires the fonts", bad.display()));
        assert!(error.contains("CRAFT_FONTS_DIR=") && error.contains(says), "{error}");
        assert_eq!((required.warning, required.source.as_str()), (None, EMPTY_TABLE));
        // Not required: the build goes on without them, and says so.
        for optional in [None, var(""), var("0")] {
            let p = plan(Some(bad.into()), optional, false);
            assert_eq!((p.error, p.source.as_str()), (None, EMPTY_TABLE));
            assert!(p.warning.is_some_and(|w| w.contains("building without craft-fonts") && w.contains(says)));
        }
    }
}

#[test]
fn a_good_folder_embeds_every_font_its_manifest_lists() {
    let dir = tempfile::tempdir().unwrap();
    checkout(dir.path());
    for required in [None, var("1")] {
        let p = plan(Some(dir.path().into()), required, false);
        assert_eq!((p.error, p.warning), (None, None));
        let lines: Vec<&str> = p.source.lines().collect();
        assert_eq!(lines.len(), 4, "{}", p.source);
        assert!(lines[1].contains(r#"family: "NanumGothic", style: "Regular", scripts: &["Kore", "Latn"], bytes: include_bytes!("#), "{}", lines[1]);
        assert!(lines[2].contains(r#"family: "BIZ UDPGothic", style: "Regular", scripts: &["Jpan"], bytes: include_bytes!("#), "{}", lines[2]);
        // The manifest and both fonts: a change to any of them builds again.
        assert_eq!(p.inputs.len(), 3);
        assert!(
            p.inputs[0].ends_with("fonts/manifest.txt")
                && p.inputs[1].ends_with("NanumGothic-Regular.ttf")
                && p.inputs[2].ends_with("BIZUDPGothic-Regular.ttf")
        );
        assert!(p.inputs.iter().all(|f| f.is_file()));
    }
    // The browser build keeps only its one web font, required or not.
    let web = plan(Some(dir.path().into()), var("1"), true);
    assert_eq!((web.error, web.warning, web.source.lines().count()), (None, None, 3));
    assert!(web.source.contains("BIZ UDPGothic") && !web.source.contains("NanumGothic"));
}

#[test]
fn without_a_folder_and_not_required_nothing_is_embedded_and_nothing_is_said() {
    for dir in [None, var("")] {
        for required in [None, var(""), var("0")] {
            let p = plan(dir.clone(), required, false);
            assert_eq!((p.error, p.warning, p.source.as_str(), p.inputs.len()), (None, None, EMPTY_TABLE, 0));
        }
    }
}
