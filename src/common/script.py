import caseconverter as cc

r = open("source.txt", "r")
w = open("kw.rs", "w")
w.write("pub enum KeyWord {\n")

for line in r:
        if not line.startswith("#define"): continue
        parts = line.split()
        kw = parts[1]
        kw = cc.pascalcase(kw)
        w.write(f"    {kw},\n")


w.write("}\n\n")

w.write("impl From<&str> for KeyWord {\n")
w.write("    fn from(s: &str) -> Self {\n")
w.write("        match s {\n")

r.close()
r = open("source.txt", "r")

for line in r:
        if not line.startswith("#define"): continue
        parts = line.split()
        kw = parts[1]
        kw = cc.pascalcase(kw)
        value = parts[2]
        w.write(f"            {value} => KeyWord::{kw},\n")

w.write("            _ => panic!(\"Invalid keyword\"),\n")
w.write("        }\n")
w.write("    }\n")
w.write("}\n")

w.write("impl From<KeyWord> for &str {\n")
w.write("    fn from(kw: KeyWord) -> Self {\n")
w.write("        match kw {\n")

r.close()
r = open("source.txt", "r")
for line in r:
    if not line.startswith("#define"): continue
    parts = line.split()
    kw = parts[1]
    kw = cc.pascalcase(kw)
    value = parts[2]
    w.write(f"            KeyWord::{kw} => {value},\n")

w.write("        }\n")
w.write("    }\n")
w.write("}\n")


r.close()
w.close()
