#include "bindings_src/hermes.h"

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  ret->status = true;
  switch (test)
  {
  case 0:
    hermes_string_dup(&ret->name, "Test Case 0");
    break;
  case 1:
    hermes_string_dup(&ret->name, "Test Case 1");
    break;
  case 2:
    hermes_string_dup(&ret->name, "Test Case 2");
    break;
  case 3:
    hermes_string_dup(&ret->name, "Test Case 3");
    break;
  case 4:
    hermes_string_dup(&ret->name, "Test Case 4");
    break;

  default:
    return false;
  }

  return true;
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  ret->status = true;
  switch (test)
  {
  case 0:
    hermes_string_dup(&ret->name, "Bench Case 0");
    break;
  case 1:
    hermes_string_dup(&ret->name, "Bench Case 1");
    break;
  case 2:
    hermes_string_dup(&ret->name, "Bench Case 2");
    break;

  default:
    return false;
  }

  return true;
}
