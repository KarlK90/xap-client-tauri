use std::sync::Arc;

use parking_lot::Mutex;
use tauri::State;
use uuid::Uuid;

use crate::xap::{
    protocol::XAPResult, EncoderPosition, KeyCode, KeyPosition, KeymapEncoderQuery,
    KeymapKeycodeQuery, XAPClient,
};

#[tauri::command]
pub(crate) async fn keycode_get(
    id: Uuid,
    arg: KeyPosition,
    state: State<'_, Arc<Mutex<XAPClient>>>,
) -> XAPResult<KeyCode> {
    state.lock().query(id, KeymapKeycodeQuery(arg))
}

#[tauri::command]
pub(crate) async fn encoder_keycode_get(
    id: Uuid,
    arg: EncoderPosition,
    state: State<'_, Arc<Mutex<XAPClient>>>,
) -> XAPResult<KeyCode> {
    state.lock().query(id, KeymapEncoderQuery(arg))
}