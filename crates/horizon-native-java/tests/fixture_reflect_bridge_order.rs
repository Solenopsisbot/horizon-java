//! `getDeclaredMethod`/`getMethod` on a covariant override pick the method
//! with the most specific return type, never the bridge, wherever the class
//! file puts it (#169 review F3).
//!
//! javac writes the bridge (`Object get()`) after the real method
//! (`String get()`), so a first-match lookup got the right answer by luck. A
//! class file with the bridge first is just as legal; this test builds one by
//! moving the bridge to the front of the method table. The expected string is
//! what `demo.Bridge.lookups()` printed on OpenJDK 25.0.5 for both orders
//! (`javac --release 21`, the reordered class file made by the same
//! transformation, two runs alike).

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;

const SOURCE: &str = r#"
package demo;

import java.lang.reflect.Method;

/** A covariant override: javac writes the bridge after the real method; the test also reorders it first. */
public class Bridge {
  public interface Src { Object get(); }
  public static class CovR implements Src { public String get() { return "c"; } }

  public static String lookups() throws Exception {
    Method declared = CovR.class.getDeclaredMethod("get");
    Method inherited = CovR.class.getMethod("get");
    return declared + "," + declared.getModifiers() + "," + declared.isBridge() + "|" + inherited + "," + inherited.isBridge()
        + "|" + declared.invoke(new CovR());
  }

  public static void main(String[] args) throws Exception { System.out.println(lookups()); }
}
"#;

const JAVA_25: &str = "public java.lang.String demo.Bridge$CovR.get(),1,false|public java.lang.String demo.Bridge$CovR.get(),false|c";

fn u2(bytes: &[u8], at: usize) -> usize {
    usize::from(u16::from_be_bytes([bytes[at], bytes[at + 1]]))
}

fn u4(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
}

/// The class file with every method whose descriptor returns `Object` moved
/// to the front of the method table (the rest keep their order).
fn bridge_first(class: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let count = u2(class, at);
    at += 2;
    let mut utf8 = vec![String::new(); count];
    let mut index = 1;
    while index < count {
        let tag = class[at];
        at += 1;
        match tag {
            1 => {
                let length = u2(class, at);
                utf8[index] = String::from_utf8_lossy(&class[at + 2..at + 2 + length]).into();
                at += 2 + length;
            }
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => at += 4,
            5 | 6 => {
                at += 8;
                index += 1;
            }
            7 | 8 | 16 | 19 | 20 => at += 2,
            15 => at += 3,
            other => panic!("constant pool tag {other}"),
        }
        index += 1;
    }
    at += 6;
    at += 2 + 2 * u2(class, at);
    // Each member: its descriptor and its bytes.
    let members = |mut at: usize| -> (usize, Vec<(String, std::ops::Range<usize>)>) {
        let count = u2(class, at);
        at += 2;
        let mut out = Vec::new();
        for _ in 0..count {
            let start = at;
            let descriptor = utf8[u2(class, at + 4)].clone();
            at += 6;
            let attributes = u2(class, at);
            at += 2;
            for _ in 0..attributes {
                at += 6 + u4(class, at + 2);
            }
            out.push((descriptor, start..at));
        }
        (at, out)
    };
    let (methods_at, _) = members(at);
    let (end, mut methods) = members(methods_at);
    methods.sort_by_key(|(descriptor, _)| !descriptor.ends_with(")Ljava/lang/Object;"));
    let mut out = class[..methods_at + 2].to_vec();
    for (_, range) in &methods {
        out.extend_from_slice(&class[range.clone()]);
    }
    out.extend_from_slice(&class[end..]);
    out
}

fn jar(tag: &str, reorder: bool) -> NativeJar {
    let root = std::env::temp_dir().join(format!(
        "horizon-native-java-bridge-order-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let source = root.join("src/demo/Bridge.java");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::create_dir_all(root.join("classes")).unwrap();
    fs::write(&source, SOURCE).unwrap();
    let javac = std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into());
    let status = Command::new(javac)
        .args(["--release", "21", "-nowarn", "-d"])
        .arg(root.join("classes"))
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed");
    let jar_path = root.join("bridge.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    for class in fs::read_dir(root.join("classes/demo")).unwrap() {
        let class = class.unwrap().path();
        let name = class.file_name().unwrap().to_string_lossy().into_owned();
        let mut bytes = fs::read(&class).unwrap();
        if reorder && name == "Bridge$CovR.class" {
            let reordered = bridge_first(&bytes);
            assert_ne!(reordered, bytes, "the bridge already came first");
            bytes = reordered;
        }
        zip.start_file(format!("demo/{name}"), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let _ = fs::remove_dir_all(&root);
    jar
}

#[test]
fn a_covariant_override_is_found_whichever_method_comes_first() {
    for (tag, reorder) in [("javac", false), ("bridge-first", true)] {
        let jar = jar(tag, reorder);
        let mut host = StdHost::new();
        let answer = jar
            .invoke_method_with_args(
                "demo.Bridge",
                "lookups",
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .unwrap_or_else(|error| panic!("{tag}: {error}"));
        assert_eq!(answer, Value::Ref(Some(JAVA_25.into())), "{tag}");
        assert_eq!(
            host.unknown_host_calls(),
            Vec::<(String, usize)>::new(),
            "{tag}"
        );
    }
}
