#include "bindings_src/hermes.h"

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void)
{
  const uint64_t cardano_launch_seconds = 1506246291;
  uint64_t elapsed = wasi_clocks_wall_clock_now().seconds - cardano_launch_seconds;
  // Saturating subtraction result
  elapsed &= -(elapsed <= cardano_launch);
  // to flioat and div

  return false;
}
