#include "bindings_src/hermes.h"
#include <string.h>
#include <stdio.h>

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }

void log_cardano_age(double days)
{
  hermes_string_t file;
  hermes_string_t msg;
  char msg_buffer[64];
  int msg_len;
  
  file = HERMES_STRING("cardano_age.c");
  msg_len = snprintf(msg_buffer, sizeof msg_buffer, "Cardano is live for %f days!", days);

  // Discarding encoding errors.
  if (msg_len < 0) {
    msg_len = 0;
  }

  msg = (hermes_string_t) { (uint8_t *)msg_buffer, msg_len };

  hermes_logging_api_log(2, &file, NULL, NULL, NULL, NULL, &msg, NULL);
}

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void)
{
  const uint64_t cardano_launch_seconds = 1506246291;
  const uint64_t seconds_in_a_day = 24 * 60 * 60;

  uint64_t elapsed_seconds;
  double elapsed_days;
  wasi_clocks_wall_clock_datetime_t now;
  
  wasi_clocks_wall_clock_now(&now);

  elapsed_seconds = (uint64_t)now.seconds - cardano_launch_seconds;
  // Saturating subtraction.
  elapsed_seconds &= -(elapsed_seconds <= cardano_launch_seconds);

  elapsed_days = (double)elapsed_seconds / seconds_in_a_day;
  log_cardano_age(elapsed_days);

  hermes_init_api_done(0);

  return true;
}
