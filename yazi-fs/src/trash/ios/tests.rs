use std::{fs, path::PathBuf, sync::Mutex};

use super::{Trash, super::TrashEntries, trash_info::TrashInfo};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

struct TestEnv {
	_guard: std::sync::MutexGuard<'static, ()>,
	path: PathBuf,
}

impl TestEnv {
	fn new(name: &str) -> Self {
		let guard = TEST_MUTEX.lock().unwrap();
		let path = std::env::temp_dir().join(format!("yazi_ios_trash_test_{}_{}", name, std::process::id()));
		let _ = fs::remove_dir_all(&path);
		let trash_root = path.join("trash_root");
		fs::create_dir_all(trash_root.join("files")).unwrap();
		fs::create_dir_all(trash_root.join("info")).unwrap();
		unsafe {
			std::env::set_var("YAZI_TEST_TRASH_ROOT", &trash_root);
		}
		Self { _guard: guard, path }
	}

	fn path(&self) -> &std::path::Path {
		&self.path
	}
}

impl Drop for TestEnv {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.path);
		unsafe {
			std::env::remove_var("YAZI_TEST_TRASH_ROOT");
		}
	}
}

#[test]
fn test_basic_file_and_dir_trash() {
	let env = TestEnv::new("basic");
	let trash = Trash::new().unwrap();

	// Test regular file
	let file_path = env.path().join("test_file.txt");
	fs::write(&file_path, "hello world").unwrap();

	trash.move_to_trash(&file_path).unwrap();
	assert!(!file_path.exists());

	let tops = trash.tops().unwrap();
	assert_eq!(tops.len(), 1);

	// Test directory
	let dir_path = env.path().join("test_dir");
	fs::create_dir(&dir_path).unwrap();
	fs::write(dir_path.join("sub.txt"), "sub content").unwrap();

	trash.move_to_trash(&dir_path).unwrap();
	assert!(!dir_path.exists());

	let tops_after = trash.tops().unwrap();
	assert_eq!(tops_after.len(), 2);
}

#[test]
fn test_basename_collision() {
	let env = TestEnv::new("collision");
	let trash = Trash::new().unwrap();

	let dir1 = env.path().join("a");
	let dir2 = env.path().join("b");
	fs::create_dir(&dir1).unwrap();
	fs::create_dir(&dir2).unwrap();

	let file1 = dir1.join("foo.txt");
	let file2 = dir2.join("foo.txt");
	fs::write(&file1, "content 1").unwrap();
	fs::write(&file2, "content 2").unwrap();

	trash.move_to_trash(&file1).unwrap();
	trash.move_to_trash(&file2).unwrap();

	let tops = trash.tops().unwrap();
	assert_eq!(tops.len(), 2);
}

#[test]
fn test_restore_success_and_conflict() {
	let env = TestEnv::new("restore");
	let trash = Trash::new().unwrap();

	let work_dir = env.path().join("work");
	fs::create_dir_all(&work_dir).unwrap();
	let file = work_dir.join("restore_me.txt");
	fs::write(&file, "restore data").unwrap();

	trash.move_to_trash(&file).unwrap();
	assert!(!file.exists());

	let tops = trash.tops().unwrap();
	assert_eq!(tops.len(), 1);

	// Restore item
	trash.restore(TrashEntries::new(tops)).unwrap();
	assert!(file.exists());
	assert_eq!(fs::read_to_string(&file).unwrap(), "restore data");

	// Test target conflict on restore
	trash.move_to_trash(&file).unwrap();
	fs::write(&file, "conflicting file").unwrap();

	let tops_conflict = trash.tops().unwrap();
	assert!(trash.restore(TrashEntries::new(tops_conflict)).is_err());
}

#[test]
fn test_metadata_security_traversal_rejection() {
	let env = TestEnv::new("security");
	let trash_root = env.path().join("trash_root");

	let malicious_info = trash_root.join("info/malicious.txt.trashinfo");
	fs::write(&malicious_info, "[Trash Info]\nPath=../../../../etc/passwd\nDeletionDate=2026-01-01T00:00:00").unwrap();

	let parsed = TrashInfo::parse(&malicious_info);
	assert!(parsed.is_err());
}
