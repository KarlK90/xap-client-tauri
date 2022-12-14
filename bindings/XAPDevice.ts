// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { XAPDeviceInfo } from "./XAPDeviceInfo";
import type { XAPKeyCodeConfig } from "./XAPKeyCodeConfig";
import type { XAPSecureStatus } from "./XAPSecureStatus";

export interface XAPDevice { id: string, info: XAPDeviceInfo, keymap: Array<Array<Array<XAPKeyCodeConfig>>>, secure_status: XAPSecureStatus, }