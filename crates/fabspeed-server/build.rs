// Recompile le serveur quand le build de l'interface change, pour que
// rust-embed intègre la dernière version de `ui/dist`.
fn main() {
    println!("cargo:rerun-if-changed=../../ui/dist");
}
