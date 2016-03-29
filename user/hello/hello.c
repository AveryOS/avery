#include <stdio.h>

static inline void print_val(intptr_t val)
{
  asm ("int $45" :: "a"(val));
}

int main(void)
{
  printf("Hello World\n");
  print_val(3);
  return 0;
}
