GET {{base_url}}/question-mark
HTTP 409
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:name-conflict"
jsonpath "$.title" == "Name conflict"
jsonpath "$.status" == 409
jsonpath "$.detail" == "The name 'example' is already in use."
jsonpath "$.*" count == 4
body not contains "private duplicate diagnostic"

GET {{base_url}}/direct
HTTP 409
Content-Type: application/problem+json
[Asserts]
jsonpath "$.type" == "urn:example:name-conflict"
jsonpath "$.title" == "Name conflict"
jsonpath "$.status" == 409
jsonpath "$.detail" == "The name 'example' is already in use."
jsonpath "$.*" count == 4
body not contains "private duplicate diagnostic"
