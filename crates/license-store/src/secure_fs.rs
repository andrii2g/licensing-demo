use license_core::{Code,Result};
use rustix::fs::{self,Mode,OFlags,AtFlags};
use std::{fs::File,io::{Read,Write},path::{Path,Component},os::unix::fs::MetadataExt};
pub struct SecureDir{file:File}
fn io(_:impl std::fmt::Debug)->Code{Code::IoError}
fn owner(uid:u32)->bool{uid==0||(cfg!(any(test,feature="dev"))&&uid==rustix::process::getuid().as_raw())}
fn component(name:&str)->Result<()>{if name.is_empty()||name=="."||name==".."||name.contains('/')||name.contains('\0'){Err(Code::IoError)}else{Ok(())}}
impl SecureDir{
    pub fn open(path:&Path)->Result<Self>{
        if !path.is_absolute(){return Err(Code::IoError);}
        let mut file=File::from(fs::open("/",OFlags::RDONLY|OFlags::DIRECTORY|OFlags::CLOEXEC,Mode::empty()).map_err(io)?);
        for c in path.components(){
            match c{
                Component::RootDir=>continue,
                Component::Normal(n)=>{
                    file=File::from(fs::openat(&file,n,OFlags::RDONLY|OFlags::DIRECTORY|OFlags::NOFOLLOW|OFlags::CLOEXEC,Mode::empty()).map_err(io)?);
                    let meta=file.metadata().map_err(io)?;
                    let temp_ancestor=cfg!(any(test,feature="dev"))&&meta.uid()==0&&meta.mode()&0o1000!=0;
                    if !owner(meta.uid())||(meta.mode()&0o022!=0&&!temp_ancestor){return Err(Code::IoError);}
                },
                _=>return Err(Code::IoError)
            }
        }
        Ok(Self{file})
    }
    pub fn create(path:&Path)->Result<Self>{
        let parent=Self::open(path.parent().ok_or(Code::IoError)?)?;
        let name=path.file_name().and_then(|x|x.to_str()).ok_or(Code::IoError)?;
        component(name)?;
        match fs::mkdirat(&parent.file,name,Mode::from_raw_mode(0o750)){Ok(())=>{},Err(rustix::io::Errno::EXIST)=>{},Err(e)=>return Err(io(e))}
        fs::fsync(&parent.file).map_err(io)?;Self::open(path)
    }
    pub fn read(&self,name:&str,limit:usize,private:bool)->Result<Vec<u8>>{
        component(name)?;
        let fd=fs::openat(&self.file,name,OFlags::RDONLY|OFlags::NOFOLLOW|OFlags::NONBLOCK|OFlags::CLOEXEC,Mode::empty())
            .map_err(|e|if e==rustix::io::Errno::NOENT{Code::LicenseMissing}else{Code::IoError})?;
        let file=File::from(fd);let m=file.metadata().map_err(io)?;
        if !m.is_file()||!owner(m.uid())||m.mode()&if private{0o077}else{0o022}!=0||m.len()>limit as u64{return Err(Code::IoError);}
        let mut bytes=Vec::new();file.take(limit as u64+1).read_to_end(&mut bytes).map_err(io)?;
        if bytes.len()>limit{return Err(Code::IoError);}Ok(bytes)
    }
    pub fn atomic(&self,name:&str,bytes:&[u8],private:bool)->Result<()>{
        self.atomic_inner(name,bytes,private,None)
    }
    fn atomic_inner(&self,name:&str,bytes:&[u8],private:bool,fault:Option<u8>)->Result<()>{
        component(name)?;
        match fs::statat(&self.file,name,AtFlags::SYMLINK_NOFOLLOW){
            Ok(s)=>{if fs::FileType::from_raw_mode(s.st_mode)!=fs::FileType::RegularFile||!owner(s.st_uid)||s.st_mode&0o022!=0{return Err(Code::IoError);}},
            Err(rustix::io::Errno::NOENT)=>{},Err(e)=>return Err(io(e))
        }
        let tmp=format!(".tmp-{}",uuid::Uuid::new_v4());let mode=if private{0o600}else{0o640};
        let result=(||{
            let mut file=File::from(fs::openat(&self.file,tmp.as_str(),OFlags::WRONLY|OFlags::CREATE|OFlags::EXCL|OFlags::NOFOLLOW|OFlags::CLOEXEC,Mode::from_raw_mode(mode)).map_err(io)?);
            let gid=fs::fstat(&self.file).map_err(io)?.st_gid;
            fs::fchown(&file,None,Some(rustix::process::Gid::from_raw(gid))).map_err(io)?;
            fs::fchmod(&file,Mode::from_raw_mode(mode)).map_err(io)?;
            file.write_all(bytes).map_err(io)?;
            if fault==Some(1){return Err(Code::IoError);}file.sync_all().map_err(io)?;
            if fault==Some(2){return Err(Code::IoError);}
            fs::renameat(&self.file,tmp.as_str(),&self.file,name).map_err(io)?;
            fs::fsync(&self.file).map_err(io)?;
            if fault==Some(3){return Err(Code::IoError);}Ok(())
        })();
        let _=fs::unlinkat(&self.file,tmp.as_str(),AtFlags::empty());result
    }
    pub fn remove(&self,name:&str)->Result<()>{
        component(name)?;
        match self.read(name,131072,false){Ok(_)=>{},Err(Code::LicenseMissing)=>return Ok(()),Err(e)=>return Err(e)}
        fs::unlinkat(&self.file,name,AtFlags::empty()).map_err(io)?;fs::fsync(&self.file).map_err(io)
    }
    pub fn lock(&self)->Result<File>{
        let fd=fs::openat(&self.file,"mutation.lock",OFlags::RDWR|OFlags::CREATE|OFlags::NOFOLLOW|OFlags::NONBLOCK|OFlags::CLOEXEC,Mode::from_raw_mode(0o600)).map_err(io)?;
        let f=File::from(fd);let m=f.metadata().map_err(io)?;
        if !m.is_file()||!owner(m.uid())||m.mode()&0o077!=0{return Err(Code::IoError);}
        fs::flock(&f,fs::FlockOperation::NonBlockingLockExclusive).map_err(io)?;Ok(f)
    }
}
#[cfg(test)]mod tests{
    use super::*;use std::os::unix::fs::{symlink,PermissionsExt};
    #[test]fn secure_files_and_atomic_failures(){
        let d=tempfile::tempdir().unwrap();let dir=SecureDir::open(d.path()).unwrap();
        dir.atomic("license.lic",b"old",false).unwrap();
        for point in [1,2]{assert!(dir.atomic_inner("license.lic",b"new",false,Some(point)).is_err());assert_eq!(dir.read("license.lic",100,false).unwrap(),b"old");}
        assert!(dir.atomic_inner("license.lic",b"new",false,Some(3)).is_err());assert_eq!(dir.read("license.lic",100,false).unwrap(),b"new");
        symlink("license.lic",d.path().join("link")).unwrap();assert!(dir.read("link",100,false).is_err());assert!(dir.atomic("link",b"x",false).is_err());
        std::fs::set_permissions(d.path().join("license.lic"),std::fs::Permissions::from_mode(0o666)).unwrap();assert!(dir.read("license.lic",100,false).is_err());
        assert!(dir.read("../outside",100,false).is_err());
        let lock=dir.lock().unwrap();assert!(dir.lock().is_err());drop(lock);assert!(dir.lock().is_ok());
        fs::mknodat(&dir.file,"pipe",fs::FileType::Fifo,Mode::from_raw_mode(0o600),0).unwrap();assert!(dir.read("pipe",100,false).is_err());
    }
}
