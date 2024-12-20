# Project goals and constraints


## Simple to use

I consider the user experience to be of utmost importance when making programs meant to be used by
people. Therefore I set it out that the CI should be dead simple to use. There should be no such thing as
spending time digging through tons of buttons to perform a simple action. There should be no time spent _to
learn_ how to use it even. Everything should be intuitive and self explanatory. One shouldn't need a PhD to
use a CI system.


## Simple to tweak

The point of this CI is to fit my specific needs, not anyone else's. And needs might change over time. Some
constraints which might have been valid at the time of starting the project might no longer hold in the future
and therefore some technical choices might not be the best anymore in the future. Changing the code to always
have something fitting my potentially changing needs should always be possible and easy-ish to
accomplish. Keeping everything simple is therefore a design constraint which impacted a few choices.


## Fast interactions

I fondly remember the day in 2008 or 2009 when I attended a presentation from someone working at Google on
their search engine.  He started its presentation saying that if you have a question and the answer is already
in your brain, the time it takes to retrieve the information is short enough to be considered quick. If you
don't, but the answer is in a book sitting on your desk, it is still fast. Next stop to find an information
would be at a local library, in your neighbourhood. And then, the next stop would be in a big library ... six
thousands two hundreds kilometers away.

Of course these were only a metaphor for what was going on in computers. The memory in the brain was akin to
cpu registers, the books on your desk was akin to cpu caches, the local library was a metaphor for the system
memory (RAM) and the big library 6200 km away was used to give an idea of how much slower accessing a hard
drive was in comparison.

He continued saying that "not doing anything" was actually one of the hardest thing to do for a human being
and that at google they had a requirement of at most 500ms between the moment a user press enter and the page
with the result shows up on their screen. This 500ms was apparently the limit someone can do nothing before
losing focus, before starting to daydream. He continued saying that out of 500ms, they considered that 100ms
was used in the network to move the user's query from its computer to the google's datacenter, and then 100ms
for the reply back. Therefore Google only had 300ms to understand the query, crawl the whole web in search of
answers, rank the web pages, collect the most prominent ones, create an html page with links to the relevant
websites and send it.

I'm personally skeptical when I hear about "social studies" since the replication crisis of 2011. Even more so
when the numbers are "round" (like 500). I came across this constant once on Wikipedia with a specific name I
can't unfortunately remember. On Wikipedia it was said to be 400ms. I also remember an article from the
Flutter development team explaining the same and putting it at one second.

I don't believe there is actually _a_ number for everyone. I believe some people can focus longer have
others. I don't even believe there is a number for each person. I expect someone who is well rested to be able
to focus longer before day dreaming than the same person being exhausted.

I am however convinced that the longer it takes for an action to complete, the least efficient someone
becomes. Therefore I set as a goal that interacting with the CI should be fast. How fast is fast enough? I
didn't really set a number here but it guided some of my technical choices.


## Easily extract data

One important design point, is that a user shall be able to easily extract a single element of data.  One
example a such data would be getting the output of one specific test compiled with gcc from distro running on
real hardware.  A user shouldn't need to wade through mountains of logs to extract only the output related to
that very specific test. A user shouldn't need to go through many buttons of a UI to get there. Ultimately,
such action should as quick to repeat on different builds as possible.


## Can run tests on embedded devices attached to a PC

This is the main use case I had for this CI. Therefore it is a no-brainer that it was a goal of the
project. Without this feature, everything else is useless. The CI should be able to compile tests, connect
somehow to a device, push the binaries there, optinionally reboot the device if necessary, execute the tests,
extract the test results and make them available.


## Automating tasks should be simple

A developer is much more likely to consider running tests on its "local" commits (before even creating a pull
request) if the procedure to do so is easy, quick and streamlined.  This help increases the software quality
by ensuring tests are executed more often and therefore issues are discovered earlier, when they are easier to
fix.

Automating a task should require trivial knowledges of tools that the software developers are already familiar
with or that can be picked up very quickly (in a matter of minutes).

## Possibility to follow the progression of a test execution live

This goal comes from the issue of having potentially long running tests, say a few hours long. If the test
produces output constantly, a developer shouldn't need to wait hours before he can see the beginning of the
output. In other words, as soon as some output is produced, a developer should be able to see it. Obvisouly
there will be some delays between the moment some output is produced and the moment it appears on the
developer's screen, but that delay should be limited to a short period and not dependent on the duration of
the test execution.
