from m import Price


def good_choice() -> Price:
    # SAFE: "a" is a valid choice
    return Price.objects.create(status="a")
