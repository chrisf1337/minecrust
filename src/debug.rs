use gl;
use gl::types::*;
use std::ffi::CString;

extern "system" fn debug_callback(
    source: GLenum,
    ty: GLenum,
    id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const GLchar,
    _user_param: *mut GLvoid,
) {
    if id == 131169 || id == 131185 || id == 131218 || id == 131204 {
        return;
    }
    eprintln!("--------------------");
    let message = unsafe { CString::from_raw(message as *mut i8).into_string().unwrap() };
    eprintln!("Debug message ({}): {}", id, message);

    match source {
        gl::DEBUG_SOURCE_API => eprintln!("Source: API"),
        gl::DEBUG_SOURCE_WINDOW_SYSTEM => eprintln!("Source: Window system"),
        gl::DEBUG_SOURCE_SHADER_COMPILER => eprintln!("Source: Shader compiler"),
        gl::DEBUG_SOURCE_THIRD_PARTY => eprintln!("Source: Third party"),
        gl::DEBUG_SOURCE_APPLICATION => eprintln!("Source: Application"),
        gl::DEBUG_SOURCE_OTHER => eprintln!("Source: Other"),
        _ => (),
    }

    match ty {
        gl::DEBUG_TYPE_ERROR => eprintln!("Type: Error"),
        gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => eprintln!("Type: Deprecated behavior"),
        gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => eprintln!("Type: Undefined behavior"),
        gl::DEBUG_TYPE_PORTABILITY => eprintln!("Type: Portability"),
        gl::DEBUG_TYPE_PERFORMANCE => eprintln!("Type: Performance"),
        gl::DEBUG_TYPE_MARKER => eprintln!("Type: Marker"),
        gl::DEBUG_TYPE_PUSH_GROUP => eprintln!("Type: Push group"),
        gl::DEBUG_TYPE_POP_GROUP => eprintln!("Type: Pop group"),
        gl::DEBUG_TYPE_OTHER => eprintln!("Type: Other"),
        _ => (),
    }

    match severity {
        gl::DEBUG_SEVERITY_HIGH => eprintln!("Severity: High"),
        gl::DEBUG_SEVERITY_MEDIUM => eprintln!("Severity: Medium"),
        gl::DEBUG_SEVERITY_LOW => eprintln!("Severity: Low"),
        gl::DEBUG_SEVERITY_NOTIFICATION => eprintln!("Severity: Notification"),
        _ => (),
    }
    eprintln!("");
}

pub fn enable_debug_output() {
    let mut flags: GLint = 0;
    unsafe {
        gl::GetIntegerv(gl::CONTEXT_FLAGS, &mut flags);
        if flags as u32 & gl::CONTEXT_FLAG_DEBUG_BIT > 0 {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(debug_callback, ::std::ptr::null());
            gl::DebugMessageControl(
                gl::DONT_CARE,
                gl::DONT_CARE,
                gl::DONT_CARE,
                0,
                ::std::ptr::null(),
                gl::TRUE,
            );
        } else {
            eprintln!("Unable to initialize debugging");
        }
    }
}
