def f(x):
    try:
        return int(x)
    except ValueError, TypeError:
        return 0
