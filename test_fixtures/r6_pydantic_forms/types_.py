from typing import Annotated

from pydantic import Field

Percent = Annotated[int, Field(ge=0, le=100)]
Code = Annotated[str, Field(max_length=4)]
