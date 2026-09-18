# Computer-use eval contract

Every production workflow gets a deterministic eval pack.

Track at minimum:

- end-to-end task success;
- success by execution tier;
- human intervention rate;
- exception rate and recovery success;
- action count and unnecessary-action rate;
- wall-clock latency;
- model/tool cost;
- unauthorized-side-effect count;
- stale-target / wrong-target count;
- retries and resumptions;
- workflow-level labor minutes displaced and verified business outcome.

A release cannot rely on demo success. Run repeated trajectories against fixtures and a controlled staging environment, then keep failing trajectories as regression cases.

For customer workflows, record the denominator explicitly. "95% success" means nothing without the task set, OS/app versions, run count, and allowed intervention policy.
