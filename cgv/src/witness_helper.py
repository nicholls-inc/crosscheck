# Runs a field's own validation on a CGV counterexample (WX-7 to WX-10 in
# intent/2026-10-21-witness-tests-spec.md). The generator writes this file with
# APP_ROOT, the checked path, defined above.

import importlib
import os
import re
import sys
import typing

import pytest

if APP_ROOT not in sys.path:  # noqa: F821
    sys.path.insert(0, APP_ROOT)  # noqa: F821

WITHHELD = (
    "CGV-WITNESS WITHHELD: a fractional counterexample against an integral field. Django's "
    "IntegerField truncates it before its validators run, and pydantic rejects every "
    "fractional value of an int field with int_from_float, so neither outcome is evidence "
    "about the bound (intent/2026-10-21-witness-tests.md)."
)


def _django_ready():
    import django
    from django.apps import apps
    from django.conf import settings

    if apps.ready:
        return
    if not os.environ.get("DJANGO_SETTINGS_MODULE") and not settings.configured:
        settings.configure(USE_TZ=True, INSTALLED_APPS=[])
    django.setup()
    for module in DJANGO_MODULES:  # noqa: F821
        _register_app(module)


def _register_app(module):
    """Give a model module that no installed app contains an app of its own,
    so that its classes can be defined."""
    from django.apps import AppConfig, apps

    if apps.get_containing_app_config(module) is not None:
        return
    label = "cgv_witness_" + re.sub(r"\W", "_", module)
    config = AppConfig.__new__(AppConfig)
    config.name = module
    config.label = label
    config.verbose_name = label
    config.module = None
    config.path = APP_ROOT  # noqa: F821
    config.apps = apps
    config.models = apps.all_models[label]
    apps.app_configs[label] = config
    apps.clear_cache()


def _field(spec):
    library, module, class_name, field_name = spec
    try:
        import django  # noqa: F401
    except ImportError:
        if library == "django":
            raise
    else:
        # A pydantic model's module may declare Django models too.
        _django_ready()
    cls = getattr(importlib.import_module(module), class_name)
    if library == "django":
        return cls._meta.get_field(field_name), cls
    return cls.model_fields[field_name], cls


def _integral(spec):
    field, _ = _field(spec)
    if spec[0] == "django":
        from django.db import models

        return isinstance(field, models.IntegerField)
    annotation = field.annotation
    return annotation is int or annotation == typing.Optional[int]


def _validate(spec, value):
    """Run the field's validation. Return None when it accepts the value, else
    (library and version, rejection codes)."""
    field, cls = _field(spec)
    if spec[0] == "django":
        import django
        from django.core.exceptions import ValidationError

        try:
            field.clean(value, None)
        except ValidationError as e:
            return f"Django {django.get_version()}", [err.code for err in e.error_list]
        return None
    import pydantic
    from pydantic import ConfigDict, TypeAdapter

    annotated = typing.Annotated[(field.annotation, *field.metadata)] if field.metadata else field.annotation
    config = ConfigDict(strict=True) if cls.model_config.get("strict") else None
    try:
        TypeAdapter(annotated, config=config).validate_python(value)
    except pydantic.ValidationError as e:
        return f"pydantic {pydantic.VERSION}", [err["type"] for err in e.errors()]
    return None


def _name(spec):
    library, module, class_name, field_name = spec
    return f"{library} field {module}.{class_name}.{field_name}"


def check_target(target, value, fractional):
    if fractional and _integral(target):
        pytest.skip(WITHHELD)
    rejected = _validate(target, value)
    if rejected is not None:
        version, codes = rejected
        pytest.fail(
            f"CGV-WITNESS REJECTED: on {version}, the {_name(target)} rejected the counterexample "
            f"{value!r} with codes {codes}. Tested evidence, on this one value, that the error's "
            "target rejects a value its source admits.",
            pytrace=False,
        )


def check_source(source, target, value, fractional):
    if fractional and _integral(target):
        pytest.skip(WITHHELD)
    rejected = _validate(source, value)
    if rejected is not None:
        version, codes = rejected
        pytest.fail(
            f"CGV-WITNESS SOURCE REJECTED: on {version}, the source {_name(source)} rejected the "
            f"counterexample {value!r} with codes {codes}, so the value is no witness.",
            pytrace=False,
        )

