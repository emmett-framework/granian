import pprint


# example of a RSGI handler that can serve partial files
# `granian --interface rsgi example_partial_file:app`
# TODO: needs to add header to advertise support for partial files
# TODO: return correct response if the range requested isn't supported
# TODO: return the whole file if the range requested is invalid
# TODO: example should serve whole directory and follow symlinks to make sure 
# TODO: document why this is better than just read/seeking in python code
# TODO: update RSGI spec with start, end parameters
async def app(scope, proto):
    assert scope.proto == 'http'
    start = None
    end = None
    status = 200
    if 'range' in scope.headers:
        http_range = scope.headers.get('range')
        start_offset, end_offset = http_range.split('=')[1].split('-')
        start = int(start_offset)
        if end_offset.isdigit():
            end = int(end_offset)
        print("start", start)
        print("end", end)
    if start:
        status = 216
    proto.response_file(
        status=status,
        headers=[
            ('content-type', 'text/plain'),
        ],
        file=__file__,
        start=start,
        end=end
    )