use anyhow::Result;

const SERVICE: &str = "packweave";

pub fn set(key: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, key)?;
    entry.set_password(value)?;
    Ok(())
}

pub fn get(key: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, key).ok()?.get_password().ok()
}

pub fn delete(key: &str) -> Result<()> {
    if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
        let _ = entry.delete_credential();
    }
    Ok(())
}
