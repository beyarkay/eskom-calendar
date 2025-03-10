import unittest
from datetime import datetime
from aggregate_historical_data import create_historical_data


class TestOverlapResolution(unittest.TestCase):

    def test_find_overlaps(self):
        # Define your expected output data
        expected_output = [
            {
                'stage': 6, 'start': datetime.fromisoformat('2023-09-04T22:00:00'),
                'finsh': datetime.fromisoformat('2023-09-05T05:00:00'),
                'source': 'https://twitter.com/CityofCT/status/1698744757000831345', 'include': 'coct'
            },
            {
                'stage': 5, 'start': datetime.fromisoformat('2023-09-04T18:00:00'),
                'finsh': datetime.fromisoformat('2023-09-04T22:00:00'),
                'source': 'https://twitter.com/CityofCT/status/1698744757000831345', 'include': 'coct'
            },
            {
                'stage': 3, 'start': datetime.fromisoformat('2023-09-04T10:00:00'),
                'finsh': datetime.fromisoformat('2023-09-04T18:00:00'),
                'source': 'https://twitter.com/CityofCT/status/1698744757000831345', 'include': 'coct'
            },
        ]

        # Call your function and get the output
        output = create_historical_data("../test-files")  # replace with your actual function name

        # Validate if output is the same as the expected output
        self.assertEqual(output, expected_output)


# Run the tests
if __name__ == '__main__':
    unittest.main()
