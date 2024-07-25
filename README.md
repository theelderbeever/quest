# Quest

A cli for going on http fetch quests. "Quests" are `GET`, `POST`, etc that are configured via a quest file with configurable variables, parameters, and headers which are used to build queries mapped to eacy to remember names.

## Install

Install is via `cargo` right now.

```
cargo install --git git@github.com/theelderbeever/quest.git
```

## Usage

A quest file, `./quests.yaml` by default, might look something like

```yaml
quests:
  - name: httpbin
    url: https://httpbin.org/${path-param}
    vars: []
    headers:
      - key: hello
        value: world
      - key: x-secret-key
        valueFrom:
          env: SUPER_SECRET
    params:
      - key: param1
        value: value
    methods:
      get:
        vars:
          - key: path-param
            value: get
        params:
          - key: get-param
            value: value
      post:
        headers:
          - key: content-type
            value: application/json
        vars:
          - key: path-param
            value: post
        params:
          - key: post-param
            value: value
        json: |
          { "hello": "world" }
```

A quick list shows that I have `GET` and `POST` available for `httpbin` and a var named `path-param` configured.

```
❯ quest ls
"./quests.yaml"
METHOD  NAME        VARS
GET     httpbin     path-param
POST    httpbin     path-param
```

Now if we perform a `GET` we will see that
- The url in `quest.yaml` has had `${path-param}` replaced with the value `get` defined in `.methods.get.vars`
- Query params from the default `.params` and `.methods.get.params` have been merged and added
- Our custom header from `.headers` has been added `hello: world` along with `x-secret-key: keepitsecretkeepissafe` which was read in from an environment variable (these can be provided via a `.env` file)
- The request has been sent and the body returned

```
❯ SUPER_SECRET=keepitsecretkeepissafe quest get httpbin | jq
{
  "args": {
    "get-param": "value",
    "param1": "value"
  },
  "headers": {
    "Accept": "*/*",
    "Accept-Encoding": "gzip, br, deflate",
    "Hello": "world",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66a1be12-771de04c1113db6c0f47de6b",
    "X-Secret-Key": "keepitsecretkeepissafe"
  },
  "origin": "76.155.80.50",
  "url": "https://httpbin.org/get?get-param=value&param1=value"
}
```

Doing the same with `POST` you can see the same things were done as above as well as the `json` field was filled in

```
❯ SUPER_SECRET=keepitsecretkeepissafe quest post httpbin | jq
{
  "args": {
    "param1": "value",
    "post-param": "value"
  },
  "data": "\"{ \\\"hello\\\": \\\"world\\\" }\"",
  "files": {},
  "form": {},
  "headers": {
    "Accept": "*/*",
    "Accept-Encoding": "gzip, br, deflate",
    "Content-Length": "26",
    "Content-Type": "application/json",
    "Hello": "world",
    "Host": "httpbin.org",
    "X-Amzn-Trace-Id": "Root=1-66a1be78-5de7497157011e01650640eb",
    "X-Secret-Key": "keepitsecretkeepissafe"
  },
  "json": "{ \"hello\": \"world\" }",
  "origin": "76.155.80.50",
  "url": "https://httpbin.org/post?param1=value&post-param=value"
}
```
