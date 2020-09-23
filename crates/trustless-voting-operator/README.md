

```
$  curl http://127.0.0.1:8080/nodes
[{"name":"reqc2","nodeId":"0x569b6ecec3c0a6b6a2be5475f5463320ab821f9a","subnet":"sgx"}]
```

## New Voting Session

```
curl --header "Content-Type: application/json" \
    --request POST \
    --data '{"contract":"aea5db67524e02a263b9339fe6667d6b577f3d4c","votingId":"1"}' \
    http://127.0.0.1:8080/sessions

```

```json
{
  "contract":"aea5db67524e02a263b9339fe6667d6b577f3d4c",
  "votingId":"1",
  "managerAddress":"1f87717ae7d69155d961c8062e8d76df32d7e612",
  "state":{
    "init":{
      "minVoters":0,
      "registrationDeadline":"2020-09-17T07:03:50.019177410Z",
      "voters":[]
    }
  }
}
```

## Register Voter

```
curl --header "Content-Type: application/json" \
    --request POST \
    --data '{"sender":"47c9a1ae6e29750b7e0ebdb0e85d8af0cb7161e4","sign":""}' \
    http://127.0.0.1:8080/sessions/1f87717ae7d69155d961c8062e8d76df32d7e612

```


## Destoring Voting Session

```curl --request DELETE http://127.0.0.1:8080/sessions/1f87717ae7d69155d961c8062e8d76df32d7e612```

```json
null
```

