use tauri::{AppHandle, Manager, State};

use crate::{
    identity::{
        device_name,
        qr::generate_pairing_qr,
        trusted_devices::{add_trusted_device, remove_trusted_device, save_trusted_devices},
        TrustedDevice,
    },
    AppState,
};

/// Return the public identity of this device (device_id, public key, name).
#[tauri::command]
pub async fn get_device_identity(state: State<'_, AppState>) -> Result<crate::identity::DeviceInfo, String> {
    let identity = state.device_identity.lock().map_err(|e| e.to_string())?;
    match identity.as_ref() {
        Some(id) => Ok(id.device_info(&device_name())),
        None => Err("Device identity not initialised".to_string()),
    }
}

/// Return an SVG QR code encoding the pairing payload for this device.
#[tauri::command]
pub async fn get_pairing_qr(state: State<'_, AppState>) -> Result<String, String> {
    let identity = state.device_identity.lock().map_err(|e| e.to_string())?;
    match identity.as_ref() {
        Some(id) => generate_pairing_qr(&id.device_info(&device_name())),
        None => Err("Device identity not initialised".to_string()),
    }
}

/// Return the list of trusted (paired) devices.
#[tauri::command]
pub async fn list_trusted_devices(state: State<'_, AppState>) -> Result<Vec<TrustedDevice>, String> {
    Ok(state
        .trusted_devices
        .lock()
        .map(|d| d.clone())
        .unwrap_or_default())
}

/// Add a device to the trusted list and persist the change.
#[tauri::command]
pub async fn add_trusted_device_cmd(
    device: TrustedDevice,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut devices = state.trusted_devices.lock().map_err(|e| e.to_string())?;
    add_trusted_device(&mut devices, device);
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_trusted_devices(&data_dir, &devices)
}

/// Remove a device from the trusted list by its `device_id` and persist.
#[tauri::command(rename_all = "camelCase")]
pub async fn remove_trusted_device_cmd(
    device_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut devices = state.trusted_devices.lock().map_err(|e| e.to_string())?;
    remove_trusted_device(&mut devices, &device_id);
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_trusted_devices(&data_dir, &devices)
}
