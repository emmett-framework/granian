import os
from granian.utils.range import parse_range_header


# Example of an RSGI handler that can serve partial files using response_file_partial
# Run with: `granian --interface rsgi example_partial_file:app`
# Test with: curl -H "Range: bytes=0-99" http://localhost:8000/
async def app(scope, protocol):
    assert scope.proto == 'http'

    # Get the file to serve (using this script as example)
    file_path = __file__
    file_size = os.path.getsize(file_path)

    # Check for Range header
    range_header = scope.headers.get('range', '')
    ranges = parse_range_header(range_header)

    if ranges:
        # Only support single ranges, return 416 for multiple ranges
        if len(ranges) > 1:
            protocol.response_empty(
                status=416,
                headers=[
                    ('content-range', f'bytes */{file_size}'),
                ],
            )
            return

        start, end = ranges[0]

        # Handle suffix ranges and open-ended ranges
        if start is not None and start < 0:
            # Suffix range like "bytes=-500" (last 500 bytes)
            start = max(0, file_size + start)
            end = file_size - 1
        elif end is None:
            # Open-ended range like "bytes=500-" (from 500 to end)
            end = file_size - 1

        # Validate range
        if start <= end and end < file_size:
            # Serve partial content
            protocol.response_file_partial(
                status=206,
                headers=[
                    ('content-type', 'text/plain'),
                    ('content-range', f'bytes {start}-{end}/{file_size}'),
                    ('content-length', str(end - start + 1)),
                    ('accept-ranges', 'bytes'),
                ],
                file=file_path,
                start=start,
                end=end,
            )
            return

        # Invalid range - return 416
        protocol.response_empty(
            status=416,
            headers=[
                ('content-range', f'bytes */{file_size}'),
            ],
        )
    else:
        # No range request - serve full file
        protocol.response_file(
            status=200,
            headers=[
                ('content-type', 'text/plain'),
                ('content-length', str(file_size)),
                ('accept-ranges', 'bytes'),
            ],
            file=file_path,
        )
