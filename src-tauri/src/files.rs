use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Serialize)]
pub struct FileMeta {
    pub name: String,
    pub size: u64,
}
#[derive(Clone, Serialize)]
pub struct Rejected {
    pub name: String,
    pub reason: String,
}
#[derive(Clone, Serialize)]
pub struct DropReport {
    pub accepted: Vec<FileMeta>,
    pub rejected: Vec<Rejected>,
}

pub fn inspect(paths: Vec<PathBuf>) -> DropReport {
    let mut result = DropReport {
        accepted: vec![],
        rejected: vec![],
    };
    if paths.len() > 100 {
        result.rejected.push(Rejected {
            name: "本次拖入".into(),
            reason: "M0 每次最多處理 100 個檔案".into(),
        });
    }
    for path in paths.into_iter().take(100) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        // Refuse UNC/device namespaces so a drag never triggers network filesystem access.
        let text = path.to_string_lossy();
        let reason = if text.starts_with(r"\\") || text.starts_with("//") {
            Some("M0 僅測試本機檔案，不讀取網路路徑")
        } else {
            match std::fs::symlink_metadata(&path) {
                Ok(m) if m.file_type().is_symlink() => Some("M0 不追蹤符號連結"),
                Ok(m) if m.is_dir() => Some("目前僅支援檔案，請先壓縮成 ZIP"),
                Ok(m) if m.is_file() => {
                    result.accepted.push(FileMeta {
                        name: name.clone(),
                        size: m.len(),
                    });
                    None
                }
                Ok(_) => Some("不支援此項目"),
                Err(_) => Some("無法讀取檔案資訊"),
            }
        };
        if let Some(reason) = reason {
            result.rejected.push(Rejected {
                name,
                reason: reason.into(),
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drop_preserves_only_basename_and_size_and_rejects_directory() {
        let root = std::env::temp_dir().join(format!("pocketdrop-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("模型.stl");
        std::fs::write(&file, b"solid test").unwrap();
        let report = inspect(vec![file.clone(), root.clone(), root.join("missing")]);
        assert_eq!(report.accepted.len(), 1);
        assert_eq!(report.accepted[0].name, "模型.stl");
        assert_eq!(report.accepted[0].size, 10);
        assert_eq!(report.rejected.len(), 2);
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("local_path"));
        assert!(!serialized.contains(&root.to_string_lossy().to_string()));
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn refuses_unc_without_access() {
        let r = inspect(vec![PathBuf::from(r"\\example.invalid\private\secret.txt")]);
        assert!(r.accepted.is_empty());
        assert!(r.rejected[0].reason.contains("網路"));
    }
}
