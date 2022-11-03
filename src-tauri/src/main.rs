#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[macro_use]
mod commands;
mod xap;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::tick;
use crossbeam_channel::{select, unbounded, Receiver, Sender};
use env_logger::Env;
use log::{error, info};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{
    plugin::{Builder, TauriPlugin},
    RunEvent, Runtime,
};
use tauri::{AppHandle, Manager};
use ts_rs::TS;
use uuid::Uuid;

use commands::*;
use xap::{XAPClient, XAPDeviceInfo, XAPResult, XAPSecureStatus};

fn shutdown_event_loop<R: Runtime>(sender: Sender<XAPEvent>) -> TauriPlugin<R> {
    Builder::new("event loop shutdown")
        .on_event(move |_, event| {
            if let RunEvent::ExitRequested { .. } = event {
                sender.send(XAPEvent::Exit).unwrap();
            }
        })
        .build()
}

pub(crate) enum XAPEvent {
    LogReceived {
        id: Uuid,
        log: String,
    },
    SecureStatusChanged {
        id: Uuid,
        secure_status: XAPSecureStatus,
    },
    NewDevice(Uuid),
    RemovedDevice(Uuid),
    RxError,
    Exit,
}

#[derive(Clone, Serialize, TS)]
#[serde(untagged)]
#[ts(export)]
pub(crate) enum FrontendEvent {
    NewDevice {
        id: String,
        device: XAPDeviceInfo,
    },
    RemovedDevice {
        id: String,
    },
    SecureStatusChanged {
        id: String,
        secure_status: XAPSecureStatus,
    },
    LogReceived {
        id: String,
        log: String,
    },
}

fn start_event_loop(
    app: AppHandle,
    state: Arc<Mutex<XAPClient>>,
    event_channel: Receiver<XAPEvent>,
) {
    let _ = std::thread::spawn(move || {
        let ticker = tick(Duration::from_millis(500));
        let state = state;
        info!("started event loop");
        'event_loop: loop {
            select! {
                recv(event_channel) -> msg => {
                    match msg {
                        Ok(XAPEvent::Exit) => {
                            info!("received shutdown signal, exiting!");
                            break 'event_loop;
                        },
                        Ok(XAPEvent::LogReceived{id, log}) => {
                            info!("LOG: {id} {log}");
                                app.emit_all("log", FrontendEvent::LogReceived{id: id.to_string(), log}).unwrap();
                        },
                        Ok(XAPEvent::SecureStatusChanged{id, secure_status}) => {
                            info!("Secure status changed: {id} - {secure_status}");
                            app.emit_all("secure-status-changed", FrontendEvent::SecureStatusChanged{id: id.to_string(), secure_status}).unwrap();
                        },
                        Ok(XAPEvent::NewDevice(id)) => {
                            if let Some(device) = state.lock().get_device(&id){
                                info!("detected new device - notifying frontend!");
                                let info = device.xap_info();
                                app.emit_all("new-device", FrontendEvent::NewDevice{id: id.to_string(), device: info.clone()}).unwrap();
                            }
                        },
                        Ok(XAPEvent::RemovedDevice(id)) => {
                            info!("removed device - notifying frontend!");
                            app.emit_all("removed-device", FrontendEvent::RemovedDevice{ id: id.to_string() }).unwrap();
                        }
                        Ok(XAPEvent::RxError) => {
                            if let Err(err) = state.lock().enumerate_xap_devices() {
                                error!("failed to enumerate XAP devices: {err}:\n {:#?}", err.source());
                            }
                        },
                        Err(err) => {
                            error!("error receiving event {err}");
                        },
                    }

                },
                recv(ticker) -> msg => {
                    match msg {
                        Ok(_) => {
                            // TODO maybe this can be done in a more resource effective manner...
                            let mut state = state.lock();
                            if let Err(err) = state.enumerate_xap_devices() {
                                error!("failed to enumerate XAP devices: {err}:\n {:#?}", err.source());
                            }
                            for device in state.get_devices() {
                                let info = device.xap_info();
                                app.emit_all("new-device", FrontendEvent::NewDevice{id: device.id().to_string(), device: info.clone()}).unwrap();
                            }
                        },
                        Err(err) => {
                            error!("failed receiving tick {err}");
                        }
                    }
                }
            }
        }
    });
}

fn main() -> XAPResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let (event_channel_tx, event_channel_rx): (Sender<XAPEvent>, Receiver<XAPEvent>) = unbounded();
    let state = Arc::new(Mutex::new(XAPClient::new(event_channel_tx.clone())?));

    tauri::Builder::default()
        .plugin(shutdown_event_loop(event_channel_tx))
        .invoke_handler(tauri::generate_handler![
            secure_lock,
            secure_unlock,
            secure_status_get,
            jump_to_bootloader,
            reset_eeprom,
            keycode_get,
            keycode_set,
            encoder_keycode_get,
            encoder_keycode_set,
            backlight_config_get,
            backlight_config_set,
            backlight_config_save,
            rgblight_config_get,
            rgblight_config_set,
            rgblight_config_save,
            rgbmatrix_config_get,
            rgbmatrix_config_set,
            rgbmatrix_config_save,
        ])
        .setup(move |app| {
            app.manage(state.clone());
            start_event_loop(app.handle(), state, event_channel_rx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
