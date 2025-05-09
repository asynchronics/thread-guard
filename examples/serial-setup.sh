#!/bin/bash

socat -d3 pty,raw,echo=0,link=/tmp/ttyS21 pty,raw,echo=0,link=/tmp/ttyS20
