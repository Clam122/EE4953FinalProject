#!/bin/bash
set -e
diesel setup
diesel migration run
exec ./keyserver