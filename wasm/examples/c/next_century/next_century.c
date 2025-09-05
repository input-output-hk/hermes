#include "bindings_src/hermes.h"
#include <string.h>

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }

void log_shutdown()
{
  hermes_string_t file;
  hermes_string_t msg;
  
  file = HERMES_STRING("next_century.c");
  msg = HERMES_STRING("Issuing shutdown...");

  hermes_logging_api_log(3, &file, NULL, NULL, NULL, NULL, &msg, NULL);
}

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void)
{
  const uint64_t jan_1_2100_seconds = 4102434000;

  wasi_clocks_wall_clock_datetime_t now;
  
  wasi_clocks_wall_clock_now(&now);
  
  // Waiting for the next century.
  if ((uint64_t)now.seconds < jan_1_2100_seconds) {
    log_shutdown();
    hermes_init_api_done(1);
  }

  return true;
}
