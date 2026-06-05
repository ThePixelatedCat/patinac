#include <stdio.h>
#include <stdlib.h>

void panic(const char* msg) {
    fprintf(stderr, "%s", msg);
    exit(EXIT_FAILURE);
}