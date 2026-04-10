#include <dynarmic/interface/A32/a32.h>
#include <dynarmic/interface/A32/coprocessor.h>
#include <dynarmic/interface/A64/a64.h>
#include <dynarmic/interface/exclusive_monitor.h>

namespace Dynarmic {

enum CompilerConstants: size_t {
    Optional_U32 = sizeof(std::optional<std::uint32_t>),
    Optional_U64 = sizeof(std::optional<std::uint64_t>),
    Optional_USize = sizeof(std::optional<std::size_t>),
    SharedPtr = sizeof(std::shared_ptr<A32::Coprocessor>),
    A32_UserConfig = sizeof(A32::UserConfig),
    A64_UserConfig = sizeof(A64::UserConfig),
};

void destroy_vec_cppstring(std::vector<std::string>* vec) {
    vec->~vector();
}

void destroy_vec_u64(std::vector<std::uint64_t>* vec) {
    vec->~vector();
}

void destroy_vec_u128(std::vector<Vector>* vec) {
    vec->~vector();
}

std::optional<std::uintptr_t> new_optional_usize(std::uintptr_t s) {
    if (s == 0) {
        return std::nullopt;
    }
    return std::optional(s);
}

std::optional<std::uint32_t> new_optional_u32(std::uint32_t s) {
    if (s == 0) {
        return std::nullopt;
    }
    return std::optional(s);
}

std::shared_ptr<A32::Coprocessor> new_coprocessor(A32::Coprocessor* ptr) {
    if (ptr == nullptr) {
        return std::shared_ptr<A32::Coprocessor>();
    }
    void* copied = malloc(sizeof(A32::Coprocessor));
    memcpy(copied, ptr, sizeof(A32::Coprocessor));
    return std::shared_ptr<A32::Coprocessor>((A32::Coprocessor*) copied, [](A32::Coprocessor* p) { free(p); });
}

A32::Jit* new_a32_jit(A32::UserConfig* conf) {
    return new A32::Jit(*conf);
}

void delete_a32_jit(A32::Jit* ptr) {
    delete ptr;
}

A64::Jit* new_a64_jit(A64::UserConfig* conf) {
    return new A64::Jit(*conf);
}

void delete_a64_jit(A64::Jit* ptr) {
    delete ptr;
}

} // namespace Dynarmic