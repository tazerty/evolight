//! Give memory the system allocator keeps after frees back to the system
//! ([`evolight_engine::memory::set_release_hook`]). The platform call lives in
//! `evolight-sysmem`, the workspace's only crate allowed `unsafe`.

/// Install the hook.
pub fn install() {
    evolight_engine::memory::set_release_hook(release);
}

fn release() {
    let _ = evolight_sysmem::release_free_memory();
}
