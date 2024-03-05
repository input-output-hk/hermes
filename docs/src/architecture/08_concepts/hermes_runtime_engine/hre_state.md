# HRE state management

It is obvious that a [Hermes Runtime Extension (*HRE*)][*HRE*] could has some internal mutable state.
This state has to be global for the entire [*Hermes engine*]
and has to be shared and consistent among all running [*Hermes applications*][*Hermes application*].

[*Hermes engine*]: ./../../05_building_block_view/hermes_engine.md#hermes-engine
[*Hermes application*]: ./../../05_building_block_view/hermes_engine.md#hermes-application
[*HRE*]: ../../05_building_block_view/hermes_engine.md#hermes-runtime-extension-hre
