use crate::path;
use anyhow::Result;
use cargo_toml::{self, Dependency, DependencyDetail, Edition, Product};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub fn prepare_manifest_file(
    template_path: impl AsRef<Path>,
    source_path: impl AsRef<Path>,
) -> Result<(PathBuf, PathBuf)> {
    // Parepare temporary directory
    let dir = path::create_temp_dir();
    let temp_manifest_path = dir.join("Cargo.toml");
    let manifest_dir = template_path.as_ref().parent().expect("should have parent");

    // Generate manifest base on template
    let template = fs::read_to_string(&template_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read manifest: {:?}\n  Error: {:?}",
            template_path.as_ref(),
            err
        );
    });
    let content = generate_manifest(&template, source_path, |rel_path| {
        path::canonicalize(&manifest_dir, &rel_path)
    });
    let mut temp_manifest = File::create(&temp_manifest_path).unwrap_or_else(|err| {
        panic!(
            "Failed to create manifest file: {:?}\n  Error: {:?}",
            temp_manifest_path, err
        );
    });
    temp_manifest.write_all(content.as_bytes())?;
    temp_manifest.flush()?;

    Ok((temp_manifest_path, dir))
}

fn generate_manifest<P, F>(template: &str, source_path: P, rel_to_abs: F) -> String
where
    P: AsRef<Path>,
    F: Fn(&str) -> PathBuf,
{
    let mut manifest =
        cargo_toml::Manifest::from_slice(template.as_bytes()).unwrap_or_else(|err| {
            panic!("Failed to parse manifest\n  Error: {:?}", err);
        });

    // Apply modifications: deps, lib
    manifest.dependencies = manifest
        .dependencies
        .into_iter()
        .map(|(crate_name, dep)| match dep.clone() {
            Dependency::Detailed(detail) => {
                if let Some(rel_path) = detail.clone().path {
                    let crate_dir = rel_to_abs(&rel_path);
                    let detail = DependencyDetail {
                        path: Some(String::from(crate_dir.to_str().unwrap())),
                        ..detail
                    };
                    (crate_name, Dependency::Detailed(detail))
                } else {
                    (crate_name, dep)
                }
            }
            _ => (crate_name, dep),
        })
        .collect();

    let source_path = String::from(source_path.as_ref().to_str().unwrap());
    manifest.lib = Some(Product {
        path: Some(source_path),
        edition: Some(Edition::E2018),
        ..Default::default()
    });

    toml::to_string(&manifest).unwrap_or_else(|err| {
        panic!("Failed to serialize manifest\n  Error: {:?}", err);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::print;
    use std::str::FromStr;

    #[test]
    fn test_generate_manifest() {
        let source_path =
            PathBuf::from_str("/home/macro-harness/projects/awesome/tests/test_macro.rs").unwrap();
        print(
            include_str!("../fixtures/Cargo.expected.toml"),
            generate_manifest(
                include_str!("../fixtures/Cargo.template.toml"),
                source_path,
                |rel_path| {
                    let rel = String::from(rel_path);
                    let name = rel.split('/').last().unwrap();
                    PathBuf::from_str(format!("/home/macro-harness/projects/{}", name).as_str())
                        .unwrap()
                },
            )
            .as_str(),
        );
    }
}
