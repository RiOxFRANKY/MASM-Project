use std::ffi::c_void;

pub type JniEnv = *mut *const *const c_void;
pub type JObject = *mut c_void;

const GET_ARRAY_LENGTH: usize = 171;
const GET_BYTE_ARRAY_REGION: usize = 200;
const SET_BYTE_ARRAY_REGION: usize = 208;

type GetArrayLength = unsafe extern "system" fn(JniEnv, JObject) -> i32;
type GetByteArrayRegion = unsafe extern "system" fn(JniEnv, JObject, i32, i32, *mut i8);
type SetByteArrayRegion = unsafe extern "system" fn(JniEnv, JObject, i32, i32, *const i8);

unsafe fn function(env: JniEnv, index: usize) -> *const c_void {
    unsafe { *(*env).add(index) }
}

pub unsafe fn read_bytes(env: JniEnv, array: JObject) -> Vec<u8> {
    unsafe {
        let length: GetArrayLength = std::mem::transmute(function(env, GET_ARRAY_LENGTH));
        let count = length(env, array).max(0);
        let mut buffer = vec![0u8; count as usize];
        let region: GetByteArrayRegion = std::mem::transmute(function(env, GET_BYTE_ARRAY_REGION));
        region(env, array, 0, count, buffer.as_mut_ptr() as *mut i8);
        buffer
    }
}

pub unsafe fn write_bytes(env: JniEnv, array: JObject, bytes: &[u8]) {
    unsafe {
        let length: GetArrayLength = std::mem::transmute(function(env, GET_ARRAY_LENGTH));
        let count = (length(env, array).max(0) as usize).min(bytes.len());
        let region: SetByteArrayRegion = std::mem::transmute(function(env, SET_BYTE_ARRAY_REGION));
        region(env, array, 0, count as i32, bytes.as_ptr() as *const i8);
    }
}
