# Round 3 D1: every form below used to overflow the extractor's stack
# (receiver resolution of `v` needed the class of `v.method()`, which needed
# the receiver of `v` again). Clean code: no findings.


def clean(value):
    value = value.strip()
    return value


def strip_only(x):
    x = x.strip()


def method_with_arg(x):
    x = x.m(1)


def literal_then_method(x):
    y = 1
    y = y.m()


def ignored_result(x):
    x = x.strip()
    return 1


def chained(x):
    x = x.strip().lower().upper()
    return x


def mutual(a, b):
    a = b.m()
    b = a.m()
    return a


def deep(x):
    return x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x


y = 1
y = y.m()
