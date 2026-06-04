#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

void panic(const char* msg) {
    fprintf(stderr, "%s", msg);
    exit(EXIT_FAILURE);
}

/// A type-erased array
///
/// The storage of an array is a contiguous block of memory with the
/// following layout:
/// `{ ArrayHeader header; T payload[header.count]; }`
typedef struct Array {
    /// A pointer to the array's payload,
    /// i.e. the address of the array's storage offset by `sizeof(ArrayHeader)`
    ///
    /// If the pointer is null, then it is assumed that the array has a 0 count and capacity
    void* payload;
} Array;

/// The header of an array
typedef struct ArrayHeader {
    /// The number of references to the array's storage.
    _Atomic(uint64_t) refc;
    /// The number of elements currently in the array
    uint64_t count;
    /// The total capacity of the array, in bytes. Should always be >= count * sizeof(T)
    uint64_t capacity;
} ArrayHeader;

static inline ArrayHeader* get_array_header(Array* array) {
    if (array->payload == NULL) return NULL;
    return (ArrayHeader*)((uint8_t*)array->payload - sizeof(ArrayHeader));
}

bool _array_equals(Array* lhs, Array* rhs, bool (*elem_equals)(void*, void*), uint64_t elem_size) {
    uint8_t* lhs_payload = (uint8_t*)lhs->payload;
    uint8_t* rhs_payload = (uint8_t*)rhs->payload;
    // True if the arrays share storage. This will also catch if both payloads are null
    if (lhs_payload == rhs_payload) return true;

    ArrayHeader* lhs_header = get_array_header(lhs);
    ArrayHeader* rhs_header = get_array_header(rhs);
    // If they have different numbers of elements, always false
    if (lhs_header->count != rhs_header->count) return false;

    // Check equality for each element individually
    for (uint64_t i = 0; i < lhs_header->count; ++i) {
        void* lhs_elem = &lhs_payload[i * elem_size];
        void* rhs_elem = &rhs_payload[i * elem_size];
        if (!elem_equals(lhs_elem, rhs_elem)) {
            return false;
        }
    }
    return true;
}