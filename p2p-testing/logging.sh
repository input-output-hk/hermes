#!/usr/bin/env bash

INFO()  { echo -e "\033[34m[INFO]\033[0m  $*"; }
OK()    { echo -e "\033[32m[OK]\033[0m    $*"; }
WARN()  { echo -e "\033[33m[WARN]\033[0m  $*"; }
ERROR() { echo -e "\033[31m[ERROR]\033[0m $*" >&2; }
STEP()  { echo -e "\n\033[1m▶ $*\033[0m"; }
