import csv
from datetime import datetime, timedelta


def shift(lst, inc):
    for i in range(0, inc):
        lst.append(lst.pop(0))
   
        
def groups_list_create(start, end, hc):
    lst = []
    for i in range(start, end + 1):
        lst.append(i)
    for ts in range(0, 100):
        lst.append(lst.pop(0))
        if lst[0] == hc:
            return lst


class NMB:
    def __init__(self):
        self.first_day_of_calendar = datetime(2023, 6, 5, 0, 00)
        self.end_date_of_calendar = datetime(2023, 9, 3, 0, 00)
        self.industrial_time_array = [0, 4, 8, 12, 16, 20]
        self.industrial24h_time_array = [0, 6]
        self.residential_time_array = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22]
        self.big_col = []

    def collect_data(self, areas, _dt_date, times, stage_start, name):
        for _i, groups in enumerate(areas):
            for group in groups:
                self.big_col.append((group, _dt_date, times, _i + stage_start, name))
        a = 1

    def industrial_loop(self, _date_dt, rot):
        print('industrial_loop')
        for times in range(0, 6):
            one = rot[0]
            shift(rot, 1)
            two = rot[0]
            shift(rot, 1)
            three = rot[0]
            shift(rot, 1)
            four = rot[0]
            shift(rot, 1)
            areas = [[one], [one, two], [one, two, three], [one, two, three, four]]
            self.collect_data(areas, _date_dt, self.industrial_time_array[times], 5, 'industrial')
            print(_date_dt, self.industrial_time_array[times])
            print('Stage 5:', one)
            print('Stage 6:', one, two)
            print('Stage 7:', one, two, three)
            print('Stage 8:', one, two, three, four)
            a = 1
        _date_dt = _date_dt + timedelta(days=1)
        return _date_dt

    def residential_loop(self, _date_dt, rot):
        print('residential_loop')
        for times in range(0, 12):
            one = rot[0]
            shift(rot, 5)
            two = rot[0]
            shift(rot, 5)
            three = rot[0]
            shift(rot, 5)
            four = rot[0]
            shift(rot, 5)
            areas = [[one], [one, two], [one, two, three], [one, two, three, four], [one, two, three, four], [one, two, three, four], [one, two, three, four], [one, two, three, four]]
            self.collect_data(areas, _date_dt, self.residential_time_array[times], 1, 'residential')
            print(_date_dt, self.residential_time_array[times])
            print('Stage 1:', one)
            print('Stage 2:', one, two)
            print('Stage 3:', one, two, three)
            print('Stage 4:', one, two, three, four)
            print('Stage 5:', one, two, three, four)
            print('Stage 6:', one, two, three, four)
            print('Stage 7:', one, two, three, four)
            print('Stage 8:', one, two, three, four)
            a = 1
        _date_dt = _date_dt + timedelta(days=1)
        return _date_dt

    def industrial24h_loop(self, _date_dt, rot):
        print('industrial24h_loop')
        _r = 2
        for times in range(0, 2):
            one = rot[0]
            shift(rot, _r)
            two = rot[0]
            shift(rot, _r)
            three = rot[0]
            shift(rot, _r)
            four = rot[0]
            shift(rot, 3)
            areas = [[one], [one, two], [one, two, three], [one, two, three, four]]
            self.collect_data(areas, _date_dt, self.industrial24h_time_array[times], 5, 'industrial24h')
            print(_date_dt, self.industrial24h_time_array[times])
            print('Stage 5:', one)
            print('Stage 6:', one, two)
            print('Stage 7:', one, two, three)
            print('Stage 8:', one, two, three, four)
        _date_dt = _date_dt + timedelta(days=1)
        rot.insert(0, rot.pop())
        return _date_dt

    def main_loop(self):
        busy = True
        while busy:
            _list = groups_list_create(1, 19, 12)
            _date_dt = self.first_day_of_calendar
            for days_loop in range(0, 20):
                for cycles_loop in range(1, 20):
                    if _date_dt > self.end_date_of_calendar:
                        break
                    _date_dt = self.residential_loop(_date_dt, _list)

            _list = groups_list_create(20, 30, 24)
            _date_dt = self.first_day_of_calendar
            for loop in range(0, 20):
                for industrial_cycle_loop in range(1, 12):
                    if _date_dt > self.end_date_of_calendar:
                        break
                    _date_dt = self.industrial_loop(_date_dt, _list)

            _list = groups_list_create(31, 38, 36)
            _date_dt = self.first_day_of_calendar
            for loop in range(0, 20):
                for industrial24h_cycle_loop in range(1, 9):
                    if _date_dt > self.end_date_of_calendar:
                        busy = False
                        break
                    _date_dt = self.industrial24h_loop(_date_dt, _list)

    def make_csv(self):
        now = datetime.now()
        for i in range(0, 32):
            nextMonth = now + timedelta(days=i)
            if nextMonth.day == now.day - 1:
                stopDate = nextMonth
                stopDate = stopDate.replace(hour=0, minute=0, second=0, microsecond=0)
                now = now.replace(hour=0, minute=0, second=0, microsecond=0)
                break
        for group in range(1, 39):
            f = open('nelson-mandela-bay-group-%s.csv' % group, 'w', newline="")
            w = csv.writer(f)
            w.writerow(['date_of_month', 'start_time', 'finsh_time', 'stage'])
            for info in self.big_col:
                if info[0] == group:
                    if info[4] == 'industrial':
                        _h = 4
                    if info[4] == 'industrial24h':
                        _h = 0
                    if info[4] == 'residential':
                        _h = 2
                    if _h == 2 or _h == 4:
                        _date_dt_start = info[1].replace(hour=info[2])
                        _date_dt_end = _date_dt_start + timedelta(hours=_h)
                        _time_str_start = _date_dt_start.strftime("%H:%M")
                        _time_str_end = _date_dt_end.strftime("%H:%M")
                    else:
                        if info[2] == 6:
                            _date_dt_start = info[1].replace(hour=info[2])
                            _date_dt_end = _date_dt_start + timedelta(hours=18)
                            _time_str_start = _date_dt_start.strftime("%H:%M")
                            _time_str_end = _date_dt_end.strftime("%H:%M")
                        else:
                            _date_dt_start = info[1].replace(hour=info[2])
                            _date_dt_end = _date_dt_start + timedelta(hours=6)
                            _time_str_start = _date_dt_start.strftime("%H:%M")
                            _time_str_end = _date_dt_end.strftime("%H:%M")

                    msg = [info[1].day, _time_str_start, _time_str_end, info[3]]
                    if now <= info[1] <= stopDate:
                        w.writerow(msg)
            f.close()


if __name__ == '__main__':
    nmb = NMB()
    nmb.main_loop()
    nmb.make_csv()
    ## References ##
    # Residential
    #  https://nelsonmandelabay.gov.za/DataRepository/Documents/loadshedding-residential-5june23-3sept23_mmyXK.pdf
    # Industrial
    #  https://www.nelsonmandelabay.gov.za/DataRepository/Documents/loadshedding-industrial-5june23-3sept23_3GeCf.pdf
    # Industrial24h
    #  https://nelsonmandelabay.gov.za/DataRepository/Documents/24-hour-schedule-5-june-to-3-september-2023_j3dWv.pdf
    

