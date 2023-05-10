#!/bin/bash
mount -o remount,group /sys/kernel/tracing/
chgrp -R tracing /sys/kernel/tracing/
chmod -R g+rwx /sys/kernel/tracing/
chmod -R g+wx /sys/kernel/tracing/events
chmod -R g+wx /sys/kernel/tracing/uprobe_events

sysctl kernel.perf_event_paranoid=-1 -w
sysctl kernel.kptr_restrict=0 -w
setcap cap_sys_rawio=ep `which rdmsr`
chgrp -R msr /dev/cpu/*/msr
chmod g+r /dev/cpu/*/msr

cpufreq-set -f 4G
echo 0 | sudo tee /sys/devices/system/cpu/cpu{2..11}/online