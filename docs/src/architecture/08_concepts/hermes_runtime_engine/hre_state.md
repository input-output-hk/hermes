# HRE state management

[Hermes Runtime Extension (HRE)][*HRE*] may possess internal mutable states.
These states must be global across the entire [*Hermes application*],
ensuring shared and consistent access among all active [*Hermes applications*][*Hermes application*].

[*Hermes engine*]: ./../../05_building_block_view/hermes_engine.md#hermes-engine
[*Hermes application*]: ./../../05_building_block_view/hermes_engine.md#hermes-application
[*HRE*]: ../../05_building_block_view/hermes_engine.md#hermes-runtime-extension-hre
