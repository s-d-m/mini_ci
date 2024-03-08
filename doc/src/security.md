# Security self assessment

Before talking about security, one needs to define a threat model and risk assessment. In my case, the threat
model is quite simple: this is a toy project meant to be used only for myself on a private environment and not
publicly accessible to anyone outside of my private network. Therefore most security protection are
unnecessary here. Nevertheless, below is a list of identified security issues and the mitigations (or lack of)
in place in the project.

## ✅ Prevention against SQL injection

The webserver receives external user-controlled inputs which ends up being used to craft `SQL` requests.
`SQL` injections are a well-known class of security issues which at its root come from not separating
parameters from the request itself and instead mixing them together through string concatenation.

To prevent this class of issue, all `SQL` requests are made through `sqlx`, a `rust` library managing
databases connections, and use the feature of providing arguments separately from the request itself.

## ✅ HTML escape everywhere (hopefully)

The "build detail" web page contains data that is produced by external applications, ultimately controlled
by the user. For example, the output of a test is given by the worker and saved in the database. A malicious
user could thus try to inject malicious code that would be interpreted by the web browser when loading a
page.

I'm unfortunately not aware of a method equivalent to the `SQL` injection prevention described above for
`HTML`. Instead of having separated `HTML` "layout" and "content" which would prevent this class of issue
completely, the user controlled inputs is first escaped, and then added to the `HTML` "layout" part.

All places where data might come from the databse has been manually checked to ensure it goes through a
a sanitisation function before getting added the the rest of the `HTML` page. However, all it takes is
to miss one single place for the security to be defeated. In a spirit of defence in depth, other
mechanisms are in place too.

## ✅ No javascript, no images, no videos, no sound, no remote fonts, no external css

The website does not use javascript on any pages. Nor does it load external assets such as images, videos or
remote fonts. Consequently, the `html` pages contain a very strict content security policy set to forbid
loading such assets. This is done using `<meta http-equiv="Content-Security-Policy" content="default-src 'none'">`

Therefore, if some user-controlled data were to be displayed without being sanitised, and tried to execute
some javascript code, or load an external resource, it would be blocked by the browser. This is valid for both
inline javascript or other resource, as well as for loading external assets.

Regarding the `CSS`, the webpage embed all the `CSS` it uses in the `HTML` header part, and the content security
policy is set to only allow that specific `CSS` which is verified by a hash.

Consequently, all security issues related to external sources loading or using assets should be prevented.

## ❌ NO HTTPS

At the time of writing, the webserver only serves pages via unsecured channels. Since the website does not
contain private data, and everything is accessible to any user, this doesn't lead to an issue of the
"information leak" type.

However, since the communication channel is not encrypted, an attacker who could modify or inject packets
in the network could therefore send a differnt page to the client and make him load that one instead.

This is certainly a big issue for a public-facing website. In this case, since the `mini_ci` is only
meant to be used by myself, for my own needs only, at home, I'm personally not concerned about this.
Technically I'm not concerned about any other potential security issue for that same reason though.

Obviously, for any more serious usage than just at-home toy project, `https` should be enabled and even
enforced with `HSTS`

## ❌ Denial of service

The CI server doesn't try to be resilient to denial of service attacks. It doesn't implement any "API
throttling" or features similar to "fail2ban" when an IP address does too many requests too quickly.  This
means it is trivial to render the CI server unusable. One simple way to do so is to keep posting job requests
quickly. Workers execute job requests in the order they were posted. Therefore this attack would force a
legitimate job request to wait for a loooong time before getting picked up.

Another way of making the CI server useless is by using the API meant for workers. When idling, a worker will
check if there are any pending task it can process. If so, it takes the task out of the pending list and starts
working on it and writes the output to the database. A denial of attack could simply be done by using the API
to keep requesting pending task and never actually executes them. This will make the tasks seen as "running"
whereas nothing will be achieved.

Yet another way to make the CI unusable, still using the worker's dedicated API, is to push data to the database,
pretending it is related to a job being currently processed. This way the legitimate job's output will be completely
scrambled and dwarved among a sea of garbage data. Preventing this issue can be done in a way similar to the
anti-CSRF tokens. Implementing this security is at odd with the "simple and easily tweakable" goals.

## ❌ Remote code execution

The point of a CI is to execute cute coming from a different place: the tests of the actual project to
run. This includes running code when compiling since the project uses CMake which let's users execute
arbitrary code at the "cmake generation" step. On top of that, tests can often execute script for legitimate
reasons, which also means there can be arbitrary code execution at the "run test" step.

Since one goal of the project was to remain simple and tweakable on one side, and used only by myself on
another side, I didn't spend any time implementing a sandbox mechanism for all the code executed by the
project to test itself. Instead, the cmake generation step, and the test execution tests are executed directly
on the same environment as the CI itself. This means there is a huge security issue trivial to use. However,
since I'm the only one who can use this CI on my personal network, and also the only one who controls the
tested project, no one can get gain access they didn't already have to anything.

## ✅ Compliance with GDPR

The website do not share data any data with any third party, nor does it uses any cookie. Consequently, there
is no need to show a banner asking for the user about its preferences about usage of its personal information.

On top of that, no personally identifiable information is saved anywhere. Not even ip addresses or user agents
are logged.

Consequently, the website complies with GDPR rules.

## ❌ Supply chain attack

`mini_ci` makes use of external `rust` crates which were selected based on reputation. Said dependencies also
have depdencies themselves. I didn't take the time to thoroughly review and vet all the external code I'm using.
This is surely a case for unwarranted trust and certainly is a risk.
