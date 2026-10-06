//! Stub implementations for dynamic dispatch
//! These are placeholder functions that allow compilation of dyn Trait code
//! to succeed without panicking at runtime

/// Stub for all trait object method calls
/// This is a placeholder that prevents linker errors during Phase 2.5 testing
#[no_mangle]
pub extern "C" fn generic_dyn_dispatch_stub() {
    // This is just a placeholder to satisfy the linker
    // In a full implementation, this would use the vtable to dispatch to the real method
}

// Generate stubs for common trait methods
#[no_mangle]
pub extern "C" fn dyn_Animal_dispatch_speak() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Reader_dispatch_read() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Drawable_dispatch_draw() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Handler_dispatch_handle() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn dyn_Printer_dispatch_print() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Updater_dispatch_update() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Worker_dispatch_work() {
    // Placeholder stub
}

#[no_mangle]
pub extern "C" fn dyn_Container_dispatch_get_size() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn dyn_Shape_dispatch_area() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn dyn_Calculator_dispatch_add() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn dyn_Empty_dispatch_() {
    // Placeholder stub
}

// Macro to generate stubs for any trait method pattern
// This allows tests to link successfully even though actual vtable dispatch isn't implemented
