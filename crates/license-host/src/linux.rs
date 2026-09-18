use crate::fingerprint::*;
use license_core::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};
pub fn bounded(path: &Path, max: usize) -> Result<String> {
    let file = std::fs::File::open(path).map_err(|_| Code::IdentityUnavailable)?;
    let mut bytes = Vec::new();
    file.take((max + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Code::IdentityUnavailable)?;
    if bytes.len() > max {
        return Err(Code::IdentityUnavailable);
    }
    String::from_utf8(bytes).map_err(|_| Code::IdentityUnavailable)
}
pub fn cpu_ranges(s: &str) -> Result<BTreeSet<u32>> {
    let mut out = BTreeSet::new();
    for part in s.trim().split(',') {
        let range = part.split('-').collect::<Vec<_>>();
        let lo = range[0]
            .parse::<u32>()
            .map_err(|_| Code::IdentityUnavailable)?;
        let hi = if range.len() == 1 {
            lo
        } else if range.len() == 2 {
            range[1]
                .parse::<u32>()
                .map_err(|_| Code::IdentityUnavailable)?
        } else {
            return Err(Code::IdentityUnavailable);
        };
        if lo > hi || hi >= 1048576 {
            return Err(Code::IdentityUnavailable);
        }
        for n in lo..=hi {
            if !out.insert(n) {
                return Err(Code::IdentityUnavailable);
            }
        }
    }
    if out.is_empty() {
        return Err(Code::IdentityUnavailable);
    }
    Ok(out)
}
fn optional(path: PathBuf, max: usize) -> Option<String> {
    bounded(&path, max)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
pub fn collect(product: &str, now: i64) -> Result<Inventory> {
    let uts = rustix::system::uname();
    let affinity = rustix::thread::sched_getaffinity(None).ok();
    let online = optional(PathBuf::from("/sys/devices/system/cpu/online"), 65536)
        .and_then(|s| cpu_ranges(&s).ok());
    let available = online
        .as_ref()
        .and_then(|set| {
            affinity.as_ref().map(|a| {
                set.iter()
                    .filter(|n| {
                        (**n as usize) < rustix::thread::CpuSet::MAX_CPU && a.is_set(**n as usize)
                    })
                    .count() as u32
            })
        })
        .filter(|n| *n > 0);
    collect_from(
        Path::new("/"),
        product,
        now,
        &uts.nodename().to_string_lossy(),
        &uts.release().to_string_lossy(),
        &uts.machine().to_string_lossy(),
        available,
    )
}
pub fn collect_from(
    root: &Path,
    product: &str,
    now: i64,
    hostname: &str,
    kernel: &str,
    arch: &str,
    available: Option<u32>,
) -> Result<Inventory> {
    identifier(product)?;
    let mid = machine_id(&bounded(&root.join("etc/machine-id"), 256)?)?;
    let dmi = optional(root.join("sys/class/dmi/id/product_uuid"), 256)
        .and_then(|s| system_uuid(&s).ok());
    let cpu_root = root.join("sys/devices/system/cpu");
    let online = optional(cpu_root.join("online"), 65536).and_then(|s| cpu_ranges(&s).ok());
    let mut packages = BTreeSet::new();
    let mut cores = BTreeSet::new();
    let mut complete = true;
    if let Some(set) = &online {
        for n in set {
            let topo = cpu_root.join(format!("cpu{n}/topology"));
            let pair = optional(topo.join("physical_package_id"), 32)
                .and_then(|s| s.parse::<u32>().ok())
                .zip(optional(topo.join("core_id"), 32).and_then(|s| s.parse::<u32>().ok()));
            if let Some((p, c)) = pair {
                packages.insert(p);
                cores.insert((p, c));
            } else {
                complete = false;
            }
        }
    } else {
        complete = false;
    }
    let model = optional(root.join("proc/cpuinfo"), 1048576)
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.split_once(':')
                    .filter(|(k, _)| k.trim() == "model name")
                    .map(|(_, v)| v.trim().to_string())
            })
        })
        .filter(|s| s.chars().count() <= 256);
    let mut nics = vec![];
    if let Ok(entries) = std::fs::read_dir(root.join("sys/class/net")) {
        for entry in entries.take(256).flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "lo" {
                continue;
            }
            if let Some(mac) = optional(entry.path().join("address"), 64) {
                let mac = mac.to_ascii_lowercase();
                let parts = mac
                    .split(':')
                    .map(|s| u8::from_str_radix(s, 16))
                    .collect::<std::result::Result<Vec<_>, _>>();
                if let Ok(b) = parts
                    && b.len() == 6
                    && b.iter().any(|x| *x != 0)
                    && b[0] & 1 == 0
                {
                    let kind = if entry.path().join("device").exists() {
                        NicKind::Physical
                    } else if std::fs::canonicalize(entry.path())
                        .ok()
                        .is_some_and(|p| p.to_string_lossy().contains("/virtual/"))
                    {
                        NicKind::Virtual
                    } else {
                        NicKind::Unknown
                    };
                    nics.push(Nic { name, mac, kind });
                }
            }
        }
    }
    nics.sort_by(|a, b| (&a.name, &a.mac).cmp(&(&b.name, &b.mac)));
    nics.truncate(64);
    let mut os = BTreeMap::new();
    if let Some(s) = optional(root.join("etc/os-release"), 16384) {
        for l in s.lines() {
            if let Some((k, v)) = l.split_once('=') {
                os.insert(
                    k.to_string(),
                    v.trim().trim_matches('"').trim_matches('\'').to_string(),
                );
            }
        }
    }
    let vendor = optional(root.join("sys/class/dmi/id/sys_vendor"), 256).unwrap_or_default();
    let product_name =
        optional(root.join("sys/class/dmi/id/product_name"), 256).unwrap_or_default();
    let virtualization = if format!("{vendor} {product_name}")
        .to_ascii_lowercase()
        .contains("vmware")
    {
        Virtualization::Vmware
    } else {
        Virtualization::Unknown
    };
    let result = Inventory {
        schema_version: 1,
        collected_at: now,
        hostname: hostname.into(),
        network_interfaces: nics,
        cpu: Cpu {
            model,
            sockets: complete.then_some(packages.len() as u32),
            cores: complete.then_some(cores.len() as u32),
            logical_processors: online.as_ref().map(|s| s.len() as u32),
            available_logical_processors: available,
        },
        os: Os {
            id: os.remove("ID").unwrap_or_else(|| "unknown".into()),
            version: os.remove("VERSION_ID").filter(|s| !s.is_empty()),
            kernel: kernel.into(),
            architecture: arch.into(),
        },
        virtualization,
        machine_id_hash: derive(product, "machine-id", &mid),
        system_uuid_hash: dmi.map(|s| derive(product, "system-uuid", &s)),
    };
    result.validate()?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_and_virtual_machine() {
        assert_eq!(cpu_ranges("0-3,8,10-11").unwrap().len(), 7);
        assert!(cpu_ranges("2-1").is_err());
        assert!(cpu_ranges("1,1").is_err());
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let put = |p: &str, s: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, s).unwrap();
        };
        put("etc/machine-id", "0123456789abcdef0123456789abcdef");
        put(
            "sys/class/dmi/id/product_uuid",
            "12345678-1234-4234-8234-123456789abc",
        );
        put("sys/class/dmi/id/sys_vendor", "VMware");
        put("sys/devices/system/cpu/online", "0-3");
        for n in 0..4 {
            put(
                &format!("sys/devices/system/cpu/cpu{n}/topology/physical_package_id"),
                &(n / 2).to_string(),
            );
            put(
                &format!("sys/devices/system/cpu/cpu{n}/topology/core_id"),
                &(n % 2).to_string(),
            );
        }
        put("sys/class/net/ens33/address", "02:50:56:01:02:03");
        put("sys/class/net/lo/address", "00:00:00:00:00:00");
        put("sys/class/net/multi/address", "01:00:00:00:00:01");
        let i = collect_from(
            root,
            "worker-suite",
            1800144000,
            "fixture",
            "fixture",
            "x86_64",
            Some(1),
        )
        .unwrap();
        assert_eq!(i.cpu.logical_processors, Some(4));
        assert_eq!(i.cpu.available_logical_processors, Some(1));
        assert_eq!(i.cpu.cores, Some(4));
        assert_eq!(i.cpu.sockets, Some(2));
        assert_eq!(i.network_interfaces.len(), 1);
        std::fs::remove_file(root.join("sys/class/dmi/id/product_uuid")).unwrap();
        let i = collect_from(
            root,
            "worker-suite",
            1800144000,
            "renamed",
            "fixture",
            "x86_64",
            Some(1),
        )
        .unwrap();
        assert!(i.system_uuid_hash.is_none());
    }
}

#[cfg(test)]
mod permissions_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn unreadable_dmi_never_degrades_host_policy() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        std::fs::create_dir_all(p.join("etc")).unwrap();
        std::fs::write(p.join("etc/machine-id"), "0123456789abcdef0123456789abcdef").unwrap();
        std::fs::create_dir_all(p.join("sys/class/dmi/id")).unwrap();
        let dmi = p.join("sys/class/dmi/id/product_uuid");
        std::fs::write(&dmi, "12345678-1234-4234-8234-123456789abc").unwrap();
        let before = collect_from(
            p,
            "worker-suite",
            1800144000,
            "physical-fixture",
            "linux",
            "x86_64",
            None,
        )
        .unwrap();
        std::fs::set_permissions(&dmi, std::fs::Permissions::from_mode(0o0)).unwrap();
        if !rustix::process::geteuid().is_root() {
            let denied = collect_from(
                p,
                "worker-suite",
                1800144000,
                "physical-fixture",
                "linux",
                "x86_64",
                None,
            )
            .unwrap();
            assert!(denied.system_uuid_hash.is_none());
            let binding = Binding {
                policy: BindingPolicy::Host,
                machine_id_hash: before.machine_id_hash.clone(),
                system_uuid_hash: before.system_uuid_hash,
            };
            assert_eq!(
                policy::binding(
                    &binding,
                    Some(&denied.machine_id_hash),
                    denied.system_uuid_hash.as_deref()
                ),
                Err(Code::IdentityUnavailable)
            );
        }
        std::fs::set_permissions(&dmi, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(p.join("etc/machine-id"), "uninitialized").unwrap();
        assert!(
            collect_from(
                p,
                "worker-suite",
                1800144000,
                "fixture",
                "linux",
                "x86_64",
                None
            )
            .is_err()
        );
    }
}
