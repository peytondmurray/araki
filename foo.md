| lockspec in path | env name passed by user | environment exists already | status | behavior |
| ---              | ---                     | ---                        | ---      | --- |
| yes | yes | yes | error | environment by that name already exists. To update, use `araki push` |
| yes | yes | no | ok | create a  new environment using the name |
| yes | no | yes |
