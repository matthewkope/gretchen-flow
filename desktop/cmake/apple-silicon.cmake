# Keep the optional Whisper backend portable across Apple Silicon generations.
# The bundled ggml native CPU probe can advertise i8mm while compiling without
# it on newer Apple Clang. Use a consistent M1-compatible baseline instead.
if(APPLE AND CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$")
    set(GGML_NATIVE OFF CACHE BOOL "Build portable CPU kernels" FORCE)
    set(GGML_CPU_ARM_ARCH "armv8.2-a+dotprod" CACHE STRING "Apple Silicon baseline" FORCE)
endif()
