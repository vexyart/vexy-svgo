/* this_file: examples/c-ffi-usage/main.c */

#include <stdio.h>
#include <stdlib.h>

typedef enum {
  Success = 0,
  InvalidInput = 1,
  ParseError = 2,
  OptimizationError = 3,
  MemoryError = 4,
  ConfigError = 5,
} VexyErrorCode;

typedef struct {
  VexyErrorCode error_code;
  char *data;
  size_t data_length;
  size_t original_size;
  size_t optimized_size;
  char *error_message;
} VexyResult;

extern VexyResult vexy_svgo_optimize_default(const char *svg_input);
extern void vexy_svgo_free_result(VexyResult result);
extern char *vexy_svgo_get_version(void);
extern void vexy_svgo_free_string(char *s);

int main(void) {
  const char *svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"><!--comment--><rect width=\"100\" height=\"100\"/></svg>";

  char *version = vexy_svgo_get_version();
  if (version != NULL) {
    printf("vexy-svgo version: %s\n", version);
    vexy_svgo_free_string(version);
  }

  VexyResult result = vexy_svgo_optimize_default(svg);
  if (result.error_code != Success) {
    fprintf(stderr, "Optimization failed (code=%d): %s\n",
            (int)result.error_code,
            result.error_message != NULL ? result.error_message : "unknown error");
    vexy_svgo_free_result(result);
    return 1;
  }

  printf("Original:  %zu bytes\n", result.original_size);
  printf("Optimized: %zu bytes\n", result.optimized_size);
  printf("Output:\n%s\n", result.data != NULL ? result.data : "");

  vexy_svgo_free_result(result);
  return 0;
}
