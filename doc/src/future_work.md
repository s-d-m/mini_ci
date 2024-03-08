# Future work

I'm not planning to continue this project as it fulfills all my goals already, and the point is to do as
little yak-shaving as necessary. However beolow is a list of features or ideas I would implement if I had
the control of time and could spend all the time I want on any thing I fancy.

This page contains a list of features which I would be working on if I wanted to continue working on `mini_ci`

## Live test output

One feature which was actually important and lead to some design decision was the ability for a user to see
the output of a test live. This is the main reason why workers keep updating the database why output produced
by tests instead of doing only one push when a test finished.

At the moment, a user can only get a live output by keeping refreshing the html page and manually scroll to
the bottom of a test output. This can obviously be easily automated thanks adding `title` attributes
everywhere relevant in the html pages but it is such a waste of time and resources (since the user will keep
loading the full page every time).

One solution to that would be for the web server to provide websocket endpoints which would stream the data
received from a worker. There might also be some other technical possibilities based on database triggers.

## Database available on the network

One of the first things on the todo list would be to move from `sqlite` to `postgres` for the database.  The
benefits are multiple here.

For one, since `postgresql` can listen to requests on the network, workers could then directly issue requests
against the database without having to go through a proxy layer implemented via special routes in the web
server. This would simplify the project a bit and should slightly increase the performances as the proxy layer and
its overhead would then vanish.

Another benefit here would be the one of performances. The official `sqlite` web page lists the use cases for
which `sqlite` is a good fit, and others which shows where it is not the best choice for the task. `mini_ci`
falls on the later category unfortunately and there should be some performance to be gained by moving to
`postgres`.

Yet another benefit in terms of performance, is that extracting the data related to one single test of a
specific build won't require anymore to retrieve the data of all tests of that build, create a webpage with
everything, transfer all those data, only discard the data of all tests we are not interested in.

The downside of this is that the database configuration would need to implement restricted user accounts for
security purposes, in particular to ensure no one can simply wipe out the database.

## Running tests in parallel

At the moment, tests are executed sequentially. If more than one instance of a hardware is available, all of
them could be used to execute separate tasks or tests concurrently. The worker would need to monitor all of
them and ensure the output of separate tests or jobs do not get intertwined

## Desktop application

Having to use a web browser to load web pages and interact with the CI, either to post a job request or see
the output of one is something I find inefficient. For one, it looked to me that loading a simple page takes
50ms with Firefox which I can't control. I consider 50ms to be a long time to wait when interacting with
computers. Bearable, sure, but not great. I'm not a web expert but the fact that firefox needs to load a page,
to parse it, extract the structure, find out how to layout `div`, which css characteristics to use to draw the
user interface, ... and do that at every page load is surely sub-optimal. The only thing that changes in the
web pages of the CI is the content itself, not the interface. Therefore having a desktop application (either
terminal or gui), where the user interface would be compiled and not interpreted every time would surely make
for faster interactions, and lower memory usage too.

On top of providing a faster user interface, a desktop application could also make use of the user's disk to
cache data from the database, essentially having a local replica, kept in sync with the CI's one.  That way,
when loading data, the app would load everything from its local copy and displays it immediately, instead of
issueing a network request and have to wait for the output before presenting it to the user. Some work would
be required to ensure cache coherency here. For example the app could first look into its cache and display
the data, while also issuing a network request concurrently to see if something changed. If something did
change, then the app would redraw the screen with the latest data. This would make for a nicer user interface
as it means faster interactions.

## Increased performance of the workers

At the moment, the code of the workers is kept quite simple, and while they push data to the database (through
the webserver) and wait for a reply confirming everything went well, they do not do anything. Therefore the
worker end up adding some delay when starting a new test because it waits for a confirmation related to a former
test.

## Running cmake and tests in a virtual machine

This would be about fixing the huge security issue of remote code execution described on the `security` page.

Simply put, the workers would start a virtual machine in which they would execute the "cmake generation" steps as
well as executing scripts (e.g. the ones handling tests) provided in the tested project.

From a user point of view, it doesn't bring any user-facing feature though.

## Caching html pages once tasks are finished

Once a job is finished, its data is fixed, and therefore loading the page related to said job should always return
the exact same html page. This is a perfect case for caching. When the web server is told a job finished, it could
then create the html page once and save it to a cache. A request for that page later on can be processed
significantly faster that point.

## HTTPS

Enabling encrypted communications is quite high in the list of important things to do for security reasons.


## Email notification

Having to check every now and then to see the status of a job is tedious. Of course this can be easily
automated thanks to the work put into providing html page with `title` everywhere necessary, but it would
be nicer to not even have to think about it. Instead of monitoring a job status, a user could receive
automatically receive an email with the job's result as soon as it becomes available.

