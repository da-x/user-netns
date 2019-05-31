extern crate libc;
extern crate regex;

use regex::Regex;
use libc::{setns, setuid, getuid, seteuid, open, O_RDONLY, CLONE_NEWNET};
use std::env::args;
use std::ffi::CString;
use std::io::{self, Write};
use std::process::exit;
use std::process::Command;

fn fail(s: String)
{
    writeln!(io::stderr(), "user-netns: {}", s).unwrap();
    exit(-1);
}

fn check_netname(name: &String) -> &String {
    let re = Regex::new(r"^[0-9a-z-]+$").unwrap();
    if !re.is_match(name) {
        fail(format!("{} is not a valid namespace", name));
    }
    return name;
}

fn check_namespace(name: &String) -> &String {
    let re = Regex::new(r"^[0-9a-z-]+$").unwrap();
    if !re.is_match(name) {
        fail(format!("{} is not a valid namespace", name));
    }
    return name;
}

fn check_ipaddr(name: &String) -> &String {
    let re = Regex::new(r"^[0-9./]+$").unwrap();
    if !re.is_match(name) {
        fail(format!("{} is not a valid ipaddr", name));
    }
    return name;
}

fn run(args: &[String], uid : u32)
{
    if args.len() < 2 {
        fail(format!("run: params not provided"));
        exit(-1);
    }

    let namespace = check_namespace(&args[0]);
    let filename = CString::new(format!("/run/netns/{}", namespace)).unwrap();

    // Try to open the given namespace
    let fd = unsafe { open(filename.as_ptr(), O_RDONLY) };
    if fd < 0 {
        fail(format!("could not open namespace {}", namespace));
        exit(-1);
    }

    // Change network namespace
    let res = unsafe { setns(fd, CLONE_NEWNET) };
    if res < 0 {
        fail(format!("setns failed"));
        exit(-1);
    }

    // Drop Admin permissions
    let res = unsafe { setuid(uid) };
    if res < 0 {
        fail(format!("priv drop failed"));
        exit(-1);
    }

    let res = unsafe { seteuid(uid) };
    if res < 0 {
        fail(format!("priv drop failed"));
        exit(-1);
    }

    let stat = Command::new(&args[1])
        .args(&args[2..])
        .status();

    match stat {
        Ok(es) => {
            match es.code() {
                Some(code) => {
                    exit(code);
                }
                None => {
                    exit(-1);
                }
            }
        }
        Err(_) => {
            exit(-1);
        }
    }
}

fn ip<I, S>(args: I)
    where I: IntoIterator<Item=S>, S: AsRef<::std::ffi::OsStr>
{
    Command::new("ip").args(args).status().expect("'ip' execution failed");
}

fn net_add(args: &[String])
{
    if args.len() < 2 {
        fail(format!("net-add: params not provided"));
        exit(-1);
    }

    let netname = check_netname(&args[0]);
    let ipaddr = check_ipaddr(&args[1]);

    ip(&["link", "add", netname, "type", "bridge"]);
    ip(&["addr", "add", ipaddr, "dev", netname]);
    ip(&["link", "set", netname, "up"]);
}

fn net_del(args: &[String])
{
    if args.len() < 1 {
        fail(format!("net-del: params not provided"));
        exit(-1);
    }

    let netname = check_netname(&args[0]);

    ip(&["link", "del", netname]);
}

fn namespace_add(args: &[String])
{
    if args.len() < 1 {
        fail(format!("namespace-add: params not provided"));
        exit(-1);
    }

    let namespace = check_namespace(&args[0]);

    ip(&["netns", "add", namespace]);
    ip(&["netns", "exec", namespace, "ifconfig", "lo", "127.0.0.1"]);
    ip(&["netns", "exec", namespace, "bash", "-c", "echo 1 > /proc/sys/net/ipv4/ip_unprivileged_port_start"]);
}

fn namespace_del(args: &[String])
{
    if args.len() < 1 {
        fail(format!("namespace-del: params not provided"));
        exit(-1);
    }

    let namespace = check_namespace(&args[0]);

    ip(&["netns", "del", namespace]);
}

fn net_link_namespace(args: &[String])
{
    if args.len() < 3 {
        fail(format!("net-link-namespace: params not provided"));
        exit(-1);
    }

    let netname = check_netname(&args[0]);
    let namespace = check_namespace(&args[1]);
    let ipaddr = check_ipaddr(&args[2]);

    let br_end = format!("{}-{}-br", netname, namespace);
    let br_end = br_end.as_str();

    let ns_end = format!("{}-{}-ns", netname, namespace);
    let ns_end = ns_end.as_str();

    ip(&["link", "add", "dev", br_end, "type", "veth", "peer", "name", ns_end]);
    ip(&["link", "set", br_end, "master", netname]);
    ip(&["link", "set", br_end, "up"]);

    ip(&["link", "set", "dev", ns_end, "netns", namespace]);
    ip(&["netns", "exec", namespace, "ip", "addr", "add", ipaddr, "dev", ns_end]);
    ip(&["netns", "exec", namespace, "ip", "link", "set", ns_end, "up"]);
}

fn net_unlink_namespace(args: &[String])
{
    if args.len() < 2 {
        fail(format!("net-unlink-namespace: params not provided"));
        exit(-1);
    }

    let netname = check_netname(&args[0]);
    let namespace = check_namespace(&args[1]);

    let br_end = format!("{}-{}-br", netname, namespace);
    let br_end = br_end.as_str();

    ip(&["link", "del", br_end, "up"]);
}

/*
   e=user-netns

   # Example usage
   $e net-add net55 192.168.55.250/24

   $e namespace-add h1
   $e namespace-add h2
   $e net-link-namespace net55 h1 192.168.55.1/24
   $e net-link-namespace net55 h2 192.168.55.2/24

   $e net-unlink-namespace net55 h1
   $e net-unlink-namespace net55 h2
   $e namespace-del h1
   $e namespace-del h2

   $e net-del net55
   */

fn main() {
    let args : Vec<String> = args().collect();

    if args.len() < 2 {
        fail(format!("not enough params"));
        exit(-1);
    }

    let cmd = args[1].clone();
    let args = &args[2..];

    // Drop Admin permissions
    let uid = unsafe { getuid() };
    unsafe { seteuid(0); };
    unsafe { setuid(0); };

    match cmd.as_str() {
        "run"                  => run(args, uid),
        "net-add"              => net_add(args),
        "net-del"              => net_del(args),
        "namespace-add"        => namespace_add(args),
        "namespace-del"        => namespace_del(args),
        "net-link-namespace"   => net_link_namespace(args),
        "net-unlink-namespace" => net_unlink_namespace(args),
        _ => {
            fail(format!("unknown command {}", cmd));
            exit(-1);
        }
    }
}
