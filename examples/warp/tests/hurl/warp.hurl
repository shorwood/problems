GET {{base_url}}/recovered
HTTP 409
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:name-conflict"
jsonpath "$.title" == "Name conflict"
jsonpath "$.status" == 409
jsonpath "$.detail" == "The name 'example' is already in use."
jsonpath "$.*" count == 4
body not contains "private duplicate diagnostic"

GET {{base_url}}/missing
HTTP 404
[Asserts]
header "Content-Type" not exists

POST {{base_url}}/recovered
HTTP 405
[Asserts]
header "Content-Type" != "application/problem+json"

GET {{base_url}}/unrelated
HTTP 500
[Asserts]
header "Content-Type" != "application/problem+json"
