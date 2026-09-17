#![forbid(unsafe_code)]
pub mod secure_fs;
pub mod identity;
pub mod install;
pub use secure_fs::SecureDir;
pub use identity::Device;
pub fn read_absolute(path:&std::path::Path,limit:usize,private:bool)->license_core::Result<Vec<u8>>{
    let parent=path.parent().ok_or(license_core::Code::IoError)?;
    let name=path.file_name().and_then(|s|s.to_str()).ok_or(license_core::Code::IoError)?;
    SecureDir::open(parent)?.read(name,limit,private)
}
