/// Detect macOS version by reading SystemVersion.plist or using sw_vers command.
fn detect_macos_version() -> String {
    // Try reading SystemVersion.plist first (more reliable)
    if let Ok(contents) = std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist") {
        // Parse ProductVersion from plist (simple regex-based extraction)
        if let Some(version) = extract_plist_version(&contents) {
            return format_macos_version(&version);
        }
    }
    
    // Fallback: try sw_vers command
    if let Ok(output) = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
    {
        if let Ok(version_str) = String::from_utf8(output.stdout) {
            let version = version_str.trim();
            return format_macos_version(version);
        }
    }
    
    // Default fallback
    "10_15_7".to_string()
}

/// Extract ProductVersion from plist XML content.
fn extract_plist_version(plist_content: &str) -> Option<&str> {
    // Simple extraction: look for <key>ProductVersion</key><string>VERSION</string>
    let key_pattern = "<key>ProductVersion</key>";
    if let Some(key_pos) = plist_content.find(key_pattern) {
        let after_key = &plist_content[key_pos + key_pattern.len()..];
        if let Some(string_start) = after_key.find("<string>") {
            let version_start = string_start + "<string>".len();
            if let Some(string_end) = after_key[version_start..].find("</string>") {
                return Some(&after_key[version_start..version_start + string_end]);
            }
        }
    }
    None
}

/// Format macOS version for User-Agent (e.g., "10.15.7" -> "10_15_7").
fn format_macos_version(version: &str) -> String {
    version.replace('.', "_")
}

/// Detect Windows version by checking registry or using PowerShell/ver command.
#[cfg(target_os = "windows")]
fn detect_windows_version() -> String {
    // Try PowerShell to get Windows version (most reliable)
    if let Ok(output) = std::process::Command::new("powershell")
        .args(["-Command", "(Get-CimInstance Win32_OperatingSystem).Version"])
        .output()
    {
        if let Ok(version_str) = String::from_utf8(output.stdout) {
            let version = version_str.trim();
            // Parse version string (e.g., "10.0.19045" -> "10.0")
            if let Some(dot_pos) = version.find('.') {
                let major = &version[..dot_pos];
                if let Some(next_dot) = version[dot_pos + 1..].find('.') {
                    let minor = &version[dot_pos + 1..dot_pos + 1 + next_dot];
                    return format!("{}.{}", major, minor);
                }
            }
        }
    }
    
    // Fallback: Try reading registry via reg command
    if let Ok(output) = std::process::Command::new("reg")
        .args(["query", "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "/v", "CurrentVersion"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // Parse registry output to extract version
            for line in output_str.lines() {
                if line.contains("CurrentVersion") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(version) = parts.last() {
                        // Registry stores version as "6.3" etc, map to User-Agent format
                        return match *version {
                            "10.0" => "10.0".to_string(),
                            "6.3" => "6.3".to_string(),  // Windows 8.1
                            "6.2" => "6.2".to_string(),  // Windows 8
                            "6.1" => "6.1".to_string(),  // Windows 7
                            _ => version.to_string(),
                        };
                    }
                }
            }
        }
    }
    
    // Default fallback for Windows 10/11
    "10.0".to_string()
}

/// Detect Windows version (non-Windows platforms return default).
#[cfg(not(target_os = "windows"))]
fn detect_windows_version() -> String {
    "10.0".to_string()
}

/// Build a realistic User-Agent string that identifies as Celeris but includes
/// platform info for better site compatibility.
/// Auto-detects OS and architecture at runtime.
pub fn user_agent() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    // Map architecture names to common User-Agent formats
    let arch_str = match arch {
        "x86_64" => "x86_64",
        "aarch64" | "arm64" => "arm64",
        "x86" => "x86",
        "arm" => "arm",
        _ => arch,
    };
    
    // Build platform-specific User-Agent string
    let platform_info = match os {
        "linux" => format!("X11; Linux {}", arch_str),
        "macos" => {
            let macos_version = detect_macos_version();
            // Format: Macintosh; Intel Mac OS X 10_15_7 (Intel) or Macintosh; ARM64 Mac OS X 10_15_7 (Apple Silicon)
            if arch == "aarch64" || arch == "arm64" {
                format!("Macintosh; ARM64 Mac OS X {}", macos_version)
            } else {
                format!("Macintosh; Intel Mac OS X {}", macos_version)
            }
        }
        "windows" => {
            let windows_version = detect_windows_version();
            let win_arch = if arch == "x86_64" || arch == "aarch64" {
                "Win64"
            } else {
                "Win32"
            };
            format!("Windows NT {}; {}; {}", windows_version, win_arch, arch_str)
        }
        "freebsd" => format!("FreeBSD {}", arch_str),
        "openbsd" => format!("OpenBSD {}", arch_str),
        "netbsd" => format!("NetBSD {}", arch_str),
        _ => format!("{} {}", os, arch_str),
    };
    
    format!("Celeris/0.1 ({})", platform_info)
}

