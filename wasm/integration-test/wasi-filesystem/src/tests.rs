use anyhow::Result;
use std::io::{Read as _, Seek, Write};

pub type TestCases = [(&'static str, fn() -> Result<()>); 6];

pub const fn test_fns() -> TestCases {
    [
        ("Write, seek then read", test_write_seek_and_read),
        ("Append", test_append),
        ("Create then delete", test_create_and_delete),
        ("Seek", test_seek),
        ("Create exclusive flags", test_create_exclusive_flags),
        ("Truncate flag", test_truncate_flag),
    ]
}

fn test_write_seek_and_read() -> Result<()> {
    let test_file_path = "test1.txt";
    let contents = b"test one, two three";

    let mut f = std::fs::File::create(test_file_path)?;
    f.write_all(contents)?;

    f.seek(std::io::SeekFrom::Start(0))?;

    let mut bs = Vec::new();
    f.read_to_end(&mut bs)?;

    if bs.as_slice() != contents {
        anyhow::bail!("Unexpected file contents");
    }

    Ok(())
}

fn test_append() -> Result<()> {
    let test_file_path = "test6.txt";
    let contents = b"test";
    let append = b"case";
    let expected = b"testcase";

    std::fs::write(test_file_path, contents)?;

    std::fs::File::options()
        .append(true)
        .open(test_file_path)?
        .write_all(append)?;

    if std::fs::read(test_file_path)? != expected {
        anyhow::bail!("Failed to append to file");
    }

    Ok(())
}

fn test_create_and_delete() -> Result<()> {
    let test_file_path = "text2.txt";

    let _ = std::fs::File::create_new(test_file_path)?;
    std::fs::remove_file(test_file_path)?;

    Ok(())
}

fn test_seek() -> Result<()> {
    let test_file_path = "test3.txt";
    let contents = b"01234567";

    let mut f = std::fs::File::create_new(test_file_path)?;
    f.write_all(contents)?;

    f.seek(std::io::SeekFrom::Start(3))?;

    let mut buf = [0u8; 1];
    f.read_exact(&mut buf)?;

    if buf[0] == contents[3] {
        Ok(())
    } else {
        anyhow::bail!("Wrong seek position");
    }
}

fn test_create_exclusive_flags() -> Result<()> {
    let test_file_path = "/tmp/test4.txt";

    std::fs::File::create_new(test_file_path)?;

    // This should fail since the file already exists
    if std::fs::File::create_new(test_file_path)
        .is_err_and(|e| matches!(e.kind(), std::io::ErrorKind::AlreadyExists))
    {
        Ok(())
    } else {
        anyhow::bail!("Expected already exists error");
    }
}

fn test_truncate_flag() -> Result<()> {
    let test_file_path = "test5.txt";
    let contents = b"test";

    std::fs::write(test_file_path, contents)?;

    if std::fs::read(test_file_path)? != contents {
        anyhow::bail!("Wrong file contents");
    }

    std::fs::File::options()
        .truncate(true)
        .open(test_file_path)?;

    if !std::fs::read(test_file_path)?.is_empty() {
        anyhow::bail!("Failed to truncate file");
    }

    Ok(())
}
