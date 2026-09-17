//! The only unsafe boundary. Callers must satisfy contracts/license_guard.h.
mod validation;
#[unsafe(no_mangle)]
pub extern "C" fn lg_abi_version()->u32{1}
/// # Safety
/// Non-null pointers address valid, nonoverlapping memory for the promised lengths.
/// output_len is aligned and writable. No allocator ownership crosses this API.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lg_validate_v1(request:*const u8,request_len:usize,output:*mut u8,output_capacity:usize,output_len:*mut usize)->i32{
    if output_len.is_null(){return -1;}
    // SAFETY: output_len validity/alignment is part of the caller contract.
    unsafe{output_len.write(0);}
    if request.is_null()||request_len==0||request_len>8192||output_capacity>isize::MAX as usize||(output.is_null()&&output_capacity!=0){return -1;}
    let result=std::panic::catch_unwind(||{
        // SAFETY: request validity is part of the caller contract; length was bounded above.
        let input=unsafe{std::slice::from_raw_parts(request,request_len)};
        let Ok(request)=validation::Request::parse(input)else{return -1;};
        let decision=validation::validate(&request);
        let Ok(bytes)=serde_json::to_vec(&decision)else{return -3;};
        // SAFETY: output_len is valid and distinct from all buffers by contract.
        unsafe{output_len.write(bytes.len());}
        if output_capacity<bytes.len(){return -2;}
        // SAFETY: non-null writable output has at least capacity bytes and does not overlap input.
        unsafe{std::ptr::copy_nonoverlapping(bytes.as_ptr(),output,bytes.len());}0
    });
    match result{Ok(code)=>code,Err(_)=>{unsafe{output_len.write(0);}-3}}
}
