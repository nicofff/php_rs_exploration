pub fn is_gif(path: &str) -> bool {
    if path.to_lowercase().ends_with(".gif") {
        return true;
    }
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut header = [0u8; 6];
        if std::io::Read::read_exact(&mut f, &mut header).is_ok() {
            return &header[..3] == b"GIF";
        }
    }
    false
}
