import FullCalendar from "@fullcalendar/react"; // must go before plugins
import dayGridPlugin from "@fullcalendar/daygrid"; // a plugin!
import timeGridPlugin from '@fullcalendar/timegrid';
import iCalendarPlugin from "@fullcalendar/icalendar";
export interface ILoadsheddingCalendar {
  eventData?: any[];
  eventCalendarName?: string;
}

function LoadsheddingCalendar({
  eventData,
  eventCalendarName,
}: ILoadsheddingCalendar) {
  var eventDataItem: any = eventData;
  if (eventCalendarName) {
    eventDataItem = {
      //Proxy URL to overcome CORS.
      url: `${process.env.REACT_APP_CALENDAR_BASE_URL}Get?calendarName=${eventCalendarName}`,
      format: "ics",
    };
  }

  return (
    <FullCalendar
      themeSystem="bootstrap5"
      plugins={[timeGridPlugin , iCalendarPlugin]}
      initialView="timeGridWeek"
      weekends={true}
      events={eventDataItem}
    />
  );
}

export default LoadsheddingCalendar;
