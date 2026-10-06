GET {{base_url}}/opaque
HTTP 409
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:name-conflict"
jsonpath "$.title" == "Name conflict"
jsonpath "$.status" == 409
jsonpath "$.detail" == "The name 'example' is already in use."
jsonpath "$.*" count == 4
body not contains "private duplicate diagnostic"

GET {{base_url}}/problem-header
HTTP 409
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:name-conflict"
jsonpath "$.title" == "Name conflict"
jsonpath "$.status" == 409
jsonpath "$.detail" == "The name 'example' is already in use."
jsonpath "$.*" count == 4
body not contains "private duplicate diagnostic"
header "x-request-id" == "example"

GET {{base_url}}/problem-status
HTTP 401
Content-Type: application/problem+json
[Asserts]
jsonpath "$.status" == 409

GET {{base_url}}/openapi.json
HTTP 200
[Asserts]
jsonpath "$.paths['/problem'].get.responses['409'].description" contains "urn:example:name-conflict"
jsonpath "$.paths['/problem'].get.responses['409'].content['application/problem+json']" exists

POST {{base_url}}/json
Content-Type: application/json
`{`
HTTP 400
Content-Type: text/plain; charset=utf-8

POST {{base_url}}/json-problem
Content-Type: application/json
`{`
HTTP 400
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:malformed-json"
jsonpath "$.title" == "Malformed JSON"
jsonpath "$.status" == 400
jsonpath "$.detail" == "The request body must contain valid JSON."
jsonpath "$.*" count == 4

POST {{base_url}}/json
Content-Type: application/json
{"name":"example"}
HTTP 200
[Asserts]
jsonpath "$.name" == "example"

POST {{base_url}}/json-problem
Content-Type: application/json
{"name":"example"}
HTTP 200
[Asserts]
jsonpath "$.name" == "example"

POST {{base_url}}/json-problem
`{}`
HTTP 415
Content-Type: text/plain; charset=utf-8

POST {{base_url}}/limited/json
Content-Type: application/json
{"name":"example"}
HTTP 413
Content-Type: text/plain; charset=utf-8

POST {{base_url}}/limited/json-problem
Content-Type: application/json
{"name":"example"}
HTTP 413
Content-Type: text/plain; charset=utf-8

GET {{base_url}}/missing
HTTP 404
[Asserts]
body == ""
