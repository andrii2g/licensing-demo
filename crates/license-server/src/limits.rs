use std::{collections::HashMap,time::{Duration,Instant}};
pub struct Limiter{entries:HashMap<String,(Instant,u32)>}
impl Default for Limiter{fn default()->Self{Self{entries:HashMap::new()}}}
impl Limiter{
    pub fn allow(&mut self,key:String,max:u32)->bool{
        let now=Instant::now();self.entries.retain(|_,(t,_)|now.duration_since(*t)<Duration::from_secs(60));
        if !self.entries.contains_key(&key)&&self.entries.len()>=10000{return false;}
        let e=self.entries.entry(key).or_insert((now,0));if e.1>=max{return false;}e.1+=1;true
    }
}
