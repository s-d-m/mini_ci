use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Deserialize, FromRow, Clone, Eq, Hash, PartialEq)]
pub(crate) struct TaskProperties {
    pub(super) id: i64,
    pub(super) status: i64,
    pub(super) ret_code: Option<i64>,
    pub(super) task_type: i64,
    pub(super) output: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskType {
    StaticAnalyser = 1,
    ClangFormat = 2,
    ClangTidy = 3,
    Tests = 4,
}

impl TaskType {
    pub(crate) fn from_i64(val: i64) -> TaskType {
        match val {
            1 => TaskType::StaticAnalyser,
            2 => TaskType::ClangFormat,
            3 => TaskType::ClangTidy,
            4 => TaskType::Tests,
            _ => panic!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Compiler {
    GccFromHardwareVendor = 1,
    GccFromDistro = 2,
}

impl Compiler {
    pub(crate) fn from_i64(val: i64) -> Compiler {
        match val {
            1 => Compiler::GccFromHardwareVendor,
            2 => Compiler::GccFromDistro,
            _ => panic!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RequiredTests {
    AllTests,
    NoTestOnlyCompile,
    // 3 left out as it is: not even compile
    AllTestExcept(String),
    OnlySpecifiedTests(String),
}

impl RequiredTests {
    pub(crate) fn try_from_tag_and_string(
        tag: i64,
        s: Option<String>,
    ) -> Result<RequiredTests, &'static str> {
        match tag {
            1 => Ok(RequiredTests::AllTests),
            2 => Ok(RequiredTests::NoTestOnlyCompile),
            3 => Err("Tests required but test_type is set to NoTestNotEvenCompile"),
            4 => {
                let err_msg = "Tests required is set to AllTestExcept some, but no tests specified";
                match s {
                    None => Err(err_msg),
                    Some(a) if a.is_empty() => Err(err_msg),
                    Some(a) => Ok(RequiredTests::AllTestExcept(a)),
                }
            }
            5 => {
                let err_msg =
                    "Tests required is set to OnlySpecified Tests, but no tests specified";
                match s {
                    None => Err(err_msg),
                    Some(a) if a.is_empty() => Err(err_msg),
                    Some(a) => Ok(RequiredTests::OnlySpecifiedTests(a)),
                }
            }
            _ => panic!(),
        }
    }
}

pub(crate) fn is_valid_git_hash(hash: &str) -> bool {
    let l = hash.len();
    (2 < l) && (l <= 64) // with the transition to sha3-256, hashes can go to 64 hexa chars
        && hash.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub(crate) enum JobStatus {
    Pending = 1,
    Running = 2,
    Success = 3,
    Failed = 4,
    Timeout = 5,
    Skipped = 6,
}

impl JobStatus {
    pub(crate) fn from_i64(value: i64) -> JobStatus {
        match value {
            1 => JobStatus::Pending,
            2 => JobStatus::Running,
            3 => JobStatus::Success,
            4 => JobStatus::Failed,
            5 => JobStatus::Timeout,
            6 => JobStatus::Skipped,
            _ => panic!(),
        }
    }
}

pub(crate) const DOCTYPE: &'static str = "<!DOCTYPE html>";

pub(crate) const URL_OF_GIT_SERVER_FOR_BROWSER_SHOWING_COMMITS: &'static str = "https://url_of_git_server_for_example_cgit/up/to/commit";

pub(crate) fn get_head_with_title(title: &str) -> String {
    let csp = "<meta http-equiv=\"Content-Security-Policy\"
content=\"default-src 'none'; style-src 'sha256-t41UEOSNtwoa6bChlZkb5ZnBfwmjo9nr4aOy5/HX55U='\">";
// this disables issuing a network request to load the favicon.
// triggers an error on the console log in edge, since loading this is against the CSP "img-src: none"
// but at least it doesn't make a network request to load the favicon. Firefox doesn't show an error
// on the console and doesn't do a network request either. Unfortunately, it is not (yet) possible to
// allow an image data with a hash in the CSP and thus prevent the errors on the console on edge.
    let no_fav_icon = "<link rel=\"icon\" href=\"data:,\">";

    let style = r###"<style>
.button-container {
 display:table;
 margin-left:auto;
 margin-right:auto
}
button,
.button,
a.button {
 position:relative;
 display:flex;
 align-items:center;
 justify-content:center;
 padding:8px 18px;
 margin-bottom:5px;
 text-align:center;
 border-radius:8px;
 border:1px solid rgba(0,0,0,0);
 appearance:none;
 cursor:pointer;
 outline:none;
}
button.outline,
.button.outline,
a.button.outline {
 background:rgba(0,0,0,0);
 box-shadow:none;
 padding:8px 18px
}
button.outline :hover,
.button.outline :hover,
a.button.outline :hover {
 transform:none;
 box-shadow:none
}
button.primary,
.button.primary,
a.button.primary {
 box-shadow:0 4px 6px rgba(50,50,93,.11),0 1px 3px rgba(0,0,0,.08)
}
button.primary:hover,
.button.primary:hover,
a.button.primary:hover {
 box-shadow:0 2px 6px rgba(50,50,93,.21),0 1px 3px rgba(0,0,0,.08)
}
button.link,
.button.link,
a.button.link {
 background:none;
 font-size:1rem
}
button.small,
.button.small,
a.button.small {
 font-size:.8rem
}
button.wide,
.button.wide,
a.button.wide {
 min-width:200px;
 padding:14px 24px
}
a.read-more,
a.read-more:hover,
a.read-more:active {
 display:inline-flex;
 background:none;
 box-shadow:none;
 padding:0;
 margin:20px 0;
 max-width:100%
}
.code-toolbar {
 margin-bottom:20px
}
.code-toolbar .toolbar-item a {
 position:relative;
 display:inline-flex;
 align-items:center;
 justify-content:center;
 padding:3px 8px;
 margin-bottom:5px;
 text-align:center;
 font-size:13px;
 font-weight:500;
 border-radius:8px;
 border:1px solid rgba(0,0,0,0);
 appearance:none;
 cursor:pointer;
 outline:none
}
.header {
 display:flex;
 flex-direction:column;
 position:relative
}
.header__inner {
 display:flex;
 align-items:center;
 justify-content:space-between
}
.header__logo {
 display:flex;
 flex:1
}
.header__logo:after {
 content:"";
 background:repeating-linear-gradient(90deg, var(--accent), var(--accent) 2px, rgba(0,0,0,0) 0, rgba(0,0,0,0) 16px);
 display:block;
 width:100%;
 right:10px
}
.header__logo a {
 flex:0 0 auto;
 max-width:100%
}
.header .menu {
 margin:20px 0
}
.header .menu__inner {
 display:flex;
 flex-wrap:wrap;
 list-style:none;
 margin:0;
 padding:0
}
.header .menu__inner li.active {
 color:var(--accent-alpha-70)
}
.header .menu__inner li:not(:last-of-type) {
 margin-right:20px;
 margin-bottom:10px;
 flex:0 0 auto
}
.header .menu__sub-inner {
 position:relative;
 list-style:none;
 padding:0;
 margin:0
}
.header .menu__sub-inner:not(:only-child) {
 margin-left:20px
}
.header .menu__sub-inner-more {
 position:absolute;
 background:var(--background);
 box-shadow:var(--shadow);
 color:#fff;
 border:2px solid;
 margin:0;
 padding:10px;
 list-style:none;
 z-index:99;
 top:35px;
 left:0
}
.header .menu__sub-inner-more-trigger {
 color:var(--accent);
 user-select:none;
 cursor:pointer
}
.header .menu__sub-inner-more li {
 margin:0;
 padding:5px;
 white-space:nowrap
}
.logo {
 display:flex;
 align-items:center;
 text-decoration:none;
 background:var(--accent);
 color:#000;
 padding:5px 10px
}
html {
 box-sizing:border-box
}
*,
*:before,
*:after {
 box-sizing:inherit
}
body {
 margin:0;
 padding:0;
 font-family:Hack,DejaVu Sans Mono,Monaco,Consolas,Ubuntu Mono,monospace;
 font-size:1rem;
 line-height:1.54;
 background-color:var(--background);
 color:var(--color);
 text-rendering:optimizeLegibility;
 -webkit-font-smoothing:antialiased;
 -webkit-overflow-scrolling:touch;
 -webkit-text-size-adjust:100%
}
@media (max-width: 683px) {
 body {
  font-size:1rem
 }
}
h1,
h2,
h3,
h4,
h5,
h6 {
 display:flex;
 align-items:center;
 font-weight:bold;
 line-height:1.3
}
h1 {
 font-size:1.4rem
}
h2 {
 font-size:1.3rem
}
h3 {
 font-size:1.2rem
}
h4,
h5,
h6 {
 font-size:1.15rem
}
a {
 color:inherit
}
img {
 display:block;
 max-width:100%
}
img.left {
 margin-right:auto
}
img.center {
 margin-left:auto;
 margin-right:auto
}
img.right {
 margin-left:auto
}
p {
 margin-bottom:20px
}
figure {
 display:table;
 max-width:100%;
 margin:25px 0
}
figure.left,
figure img {
 margin-right:auto
}
figure.center,
figure img {
 margin-left:auto;
 margin-right:auto
}
figure.right,
figure img {
 margin-left:auto
}
figure figcaption {
 font-size:14px;
 padding:5px 10px;
 margin-top:5px;
 background:var(--accent);
 color:var(--background)
}
figure figcaption.left {
 text-align:left
}
figure figcaption.center {
 text-align:center
}
figure figcaption.right {
 text-align:right
}
code {
 font-family:Hack,DejaVu Sans Mono,Monaco,Consolas,Ubuntu Mono,monospace;
 font-feature-settings:normal;
 background:var(--accent-alpha-20);
 padding:1px 6px;
 margin:0 2px;
 font-size:.95rem
}
pre {
 font-family:Hack,DejaVu Sans Mono,Monaco,Consolas,Ubuntu Mono,monospace;
 padding:20px;
 font-size:.95rem;
 overflow:auto;
 border-top:1px solid rgba(255,255,255,.1);
 border-bottom:1px solid rgba(255,255,255,.1)
}
@media (max-width: 683px) {
 pre {
  white-space:pre-wrap;
  word-wrap:break-word
 }
}
pre code {
 padding:0;
 margin:0;
 background:none
}
blockquote {
 border-top:1px solid var(--accent);
 border-bottom:1px solid var(--accent);
 margin:40px 0;
 padding:25px
}
@media (max-width: 683px) {
 blockquote {
  padding-right:0
 }
}
blockquote:before {
 content:"ö";
 font-family:Georgia,serif;
 font-size:3.875rem;
 position:absolute;
 left:-40px;
 top:-20px
}
blockquote p:first-of-type {
 margin-top:0
}
blockquote p:last-of-type {
 margin-bottom:0
}
blockquote p {
 position:relative
}
blockquote p:before {
 content:">";
 display:block;
 position:absolute;
 left:-25px;
 color:var(--accent)
}
table {
 table-layout:fixed;
 border-collapse:collapse;
 width:100%;
 margin:40px 0
}
table,
th,
td {
 border:1px dashed var(--accent);
 padding:10px
}
th {
 color:var(--accent)
}
ul,
ol {
 margin-left:30px;
 padding:0
}
ul li,
ol li {
 position:relative
}
@media (max-width: 683px) {
 ul,
 ol {
  margin-left:20px
 }
}
ol ol {
 list-style-type:lower-alpha
}
.container {
 display:flex;
 flex-direction:column;
 padding:40px;
 max-width:864px;
 min-height:100vh;
 margin:0 auto
}
@media (max-width: 683px) {
 .container {
  padding:20px
 }
}
.content {
 display:flex
}
hr {
 width:100%;
 border:none;
 background:var(--border-color);
 height:1px
}
.hidden {
 display:none
}
.posts {
 width:100%;
 margin:0 auto
}
.post {
 width:100%;
 text-align:left;
 margin:20px auto;
 padding:20px 0
}
@media (max-width: 899px) {
 .post {
  max-width:660px
 }
}
.post:not(:last-of-type) {
 border-bottom:1px solid var(--border-color)
}
.post .post-meta-inline,
.post .post-meta {
 font-size:1rem;
 margin-bottom:10px;
 color:var(--accent-alpha-70)
}
.post-meta-inline {
 display:inline
}
.post-title {
 --border: 2px dashed var(--accent);
 position:relative;
 color:var(--accent);
 margin:0 0 15px;
 padding-bottom:15px;
 border-bottom:var(--border);
 font-weight:normal
}
.post-title a {
 text-decoration:none
}
.post .post-tags-inline,
.post .post-tags {
 margin-bottom:20px;
 font-size:1rem;
 opacity:.5
}
.post-tags {
 display:block
}
.post-tags-inline {
 display:inline
}
@media (max-width: 683px) {
 .post-tags-inline {
  display:block
 }
}
.post-content {
 margin-top:30px
}
.post-cover {
 border:20px solid var(--accent);
 background:rgba(0,0,0,0);
 margin:40px 0;
 padding:20px
}
@media (max-width: 683px) {
 .post-cover {
  padding:10px;
  border-width:10px
 }
}
.post ul {
 list-style:none
}
.post ul li:before {
 content:"?";
 position:absolute;
 left:-20px;
 color:var(--accent)
}
.post--regulation h1 {
 justify-content:center
}
.post--regulation h2 {
 justify-content:center;
 margin-bottom:10px
}
.post--regulation h2+h2 {
 margin-top:-10px;
 margin-bottom:20px
}
.post-list .post-date {
 color:var(--accent-alpha-70);
 text-decoration:none
}
.post-list a {
 text-decoration:none
}
.post-list .post-list-title {
 text-decoration:underline
}
.post-list .post-tag {
 text-decoration:underline
}
.pagination {
 margin-top:50px
}
.pagination__title {
 display:flex;
 text-align:center;
 position:relative;
 margin:100px 0 20px
}
.pagination__title-h {
 text-align:center;
 margin:0 auto;
 padding:5px 10px;
 background:var(--background);
 font-size:.8rem;
 text-transform:uppercase;
 letter-spacing:.1em;
 z-index:1
}
.pagination__title hr {
 position:absolute;
 left:0;
 right:0;
 width:100%;
 margin-top:15px;
 z-index:0
}
.pagination__buttons {
 display:flex;
 align-items:center;
 justify-content:center
}
@media (max-width: 683px) {
 .pagination__buttons {
  flex-direction:column
 }
}
.button {
 position:relative;
 display:inline-flex;
 align-items:center;
 justify-content:center;
 font-size:1rem;
 border-radius:8px;
 max-width:40%;
 padding:0;
 cursor:pointer;
 appearance:none
}
@media (max-width: 683px) {
 .button {
  max-width:80%
 }
}
.button+.button {
 margin-left:10px
}
.button a {
 display:flex;
 padding:8px 16px;
 text-overflow:ellipsis;
 white-space:nowrap;
 overflow:hidden
}
.button__text {
 text-overflow:ellipsis;
 white-space:nowrap;
 overflow:hidden
}
.footer {
 padding:40px 0;
 flex-grow:0;
 opacity:.5
}
.footer__inner {
 display:flex;
 align-items:center;
 justify-content:space-between;
 margin:0;
 width:760px;
 max-width:100%
}
@media (max-width: 899px) {
 .footer__inner {
  flex-direction:column
 }
}
.footer a {
 color:inherit
}
.footer .copyright {
 display:flex;
 flex-direction:row;
 align-items:center;
 font-size:1rem;
 color:var(--light-color-secondary)
}
.footer .copyright--user {
 margin:auto;
 text-align:center
}
.footer .copyright>*:first-child:not(:only-child) {
 margin-right:10px
}
@media (max-width: 899px) {
 .footer .copyright>*:first-child:not(:only-child) {
  border:none;
  padding:0;
  margin:0
 }
}
@media (max-width: 899px) {
 .footer .copyright {
  flex-direction:column;
  margin-top:10px
 }
}
@media (max-width: 899px) {
 .footer .copyright-theme-sep {
  display:none
 }
}
@media (max-width: 899px) {
 .footer .copyright-theme {
  font-size:.75rem
 }
	}

:root {
 --accent: rgb(255,98,102);
 --accent-alpha-70: rgba(255,98,102,.7);
 --accent-alpha-20: rgba(255,98,102,.2);
 --background: #221F29;
 --color: white;
 --border-color: rgba(255, 255, 255, .1)
}

.Running {
 background-color: #34344a;
}

.Pending {
 background-color: #2d2929;
}

.Failed {
 background-color: #5E2121;
}

.Timeout {
 background-color: #312525;
}

.Success {
 background-color: #1E481E;
}

.Skipped {
 background-color: #4a3f01;
}

.link_button {
    -webkit-border-radius: 4px;
    -moz-border-radius: 4px;
    border-radius: 4px;
    border: solid 1px #20538D;
    text-shadow: 0 -1px 0 rgba(0, 0, 0, 0.4);
    -webkit-box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.4), 0 1px 1px rgba(0, 0, 0, 0.2);
    -moz-box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.4), 0 1px 1px rgba(0, 0, 0, 0.2);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.4), 0 1px 1px rgba(0, 0, 0, 0.2);
    background: #4479BA;
    color: #FFF;
    padding: 8px 12px;
    text-decoration: none;
}

.display_block {
  display: block;
}

.display_inline_block {
  display: inline-block;
}


.center_text {
  text-align: center;
}</style>"###;
    format!("<head><meta charset=\"UTF-8\"><title>{title}</title>{csp}{no_fav_icon}{style}</head>")
}