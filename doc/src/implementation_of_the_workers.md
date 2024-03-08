# Implementation of the workers

## Language choice

There are no technical reasons to force the same technologies on the web server side as in the workers, however
`rust` fits the constraints of both, therefore there is also no reason to use two different programming
languages here either. For simplicity reason, `rust` stayed.

## Picking up a task

The tasks a worker can execute depend on the configuration of the machine itself. For example, if a static
analyser isn't installed on one machine, the worker executing on that machine can't obvioulsy perform the
task of running the static analyser on the developer's code.

Therefore before assigning a task to a worker, it is necessary to know what the worker can and cannot do. For
simplicity reason, the choice was made to have the worker poll for pending tasks. It is therefore up to the
worker to tell what it can do when looking a task. This capability should be configured per-worker since it
ultimately depends on the configuration of the machine. The configuration is for now hard-coded in the
code. Changing the configuration requires therefore to change the code and recompile. Certainly not pretty,
but good enough for my needs of the moment.

## Executing a task

The workers are really simple, do not implement any security feature, and rely on hard-coded knowledge from
the project codebase it executes things on. For example, to execute a task, a worker will:

1. retrieve the commit hash and the task to perform from the database
2. `git clone` that commit
3. either:
    - run a script from the cloned folder for tasks such as `formatting code` or `static analysis`.
    - call `cmake generate <some parameters> && cmake build && ctest` in the project folder

This keeps the worker's code simple however it brings two notable issues:

- huge security issue of type remote code execution (more on that on the [Self security assessment page](./security.md))
- no backforward/forward compatibility

_Backward/forward compatibility issue_ means that since the knowledge to execute a task is hardcoded
to run a script, the day the file name of those scripts changes, or take different arguments, the CI
will need to be adapted.

## Reporting tasks that will be executed

When running tests, the worker finds itself the exact list of test to run by running `ctest --show-only=human`
and potentially filtering out tests the user asked not to execute. It then reports the exact list
into the database. This is how it is possible for the webserver to list the tests that gets executed.

## Tests execution

Tests are executed sequentially even more than one hardware resource is available. This is obviously
suboptimal and the only reason it is that way is because I didn't spend time implementing this feature.

## Reporting data constantly to the database

When executing a task, the worker will execute long-running commands in the background, keep reading the
standard output and standard error produced by said commands and constantly update the database with it.

At the moment, only long running commands are executed that way. A few commands are still executed
synchronously and the worker wait for them to finish before reporting their output. Those are:
- `git clone`
- `git remote update`
- `cmake generate stage`

These commands execute under two seconds. The user experience is therefore not too negatively affected.
The only reason why these commands are executed that way is only due to lack of time to turn them to
background commands after an initial implementation.

Since the database is not available through the network, any access to it is done through special routes
provided by the webserver.

In order to avoid excessive network I/O, the worker will try to batch outputs, such that several lines of
`stdout/stderr` can be appended in the database with only one network request.

Importantly, the worker sanitises what it reads on `stdout/stderr` such that what is saved in the database
is guaranteed to be valid `utf-8`, meaning the inputs might be slightly silently modified in the process.

## Taking a worker out

When a developer needs to troubleshoot some issue, it is common that he will need an exclusive access to the
hardware. Since a worker is consistently looking for picking up a task to execute and run command on the same
hardware, there is a contention situation. To resolve this, there must be a way to tell a worker to
temporarily stop picking up new tasks.

This feature is implemented through `unix signals`. When a developer needs to shut off a worker so he can have
full control of the machine, he will send a signal `SIGINT` or `SIGTERM` to the worker. Upon receiving it, the
worker will acknowledge it by writing a message on the console, and finish its current task before exiting.
Sending a second signal will order the worker to abort its ongoing task to immediately relinquish resources.

A worker can then later be restarted normally.

## Missing feature from the worker

Besides the lack of parallelism when running tests, one other missing feature here is that the only output the
worker reports in the database is what the executed commands produce on their `stdout/stderr`. Any other
artifiacts, such as files produced on the file system, are simply discarded at the end of a task
execution. Again, this was good enough for my needs.
