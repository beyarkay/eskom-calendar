import FullCalendar from "@fullcalendar/react"; // must go before plugins
import dayGridPlugin from "@fullcalendar/daygrid"; // a plugin!
import iCalendarPlugin from "@fullcalendar/icalendar";
export interface IThemeToggle {
  onToggle?: (evt?: any) => void;
  eventData?: any[];
}

function LoadsheddingCalendar({ onToggle, eventData }: IThemeToggle) {
  let icalEvents = {
    url:
      "http://localhost:3000/eskom-calendar/cp15.ics",
    format: "ics"
  };

  return (
    <FullCalendar
      plugins={[dayGridPlugin, iCalendarPlugin]}
      initialView="dayGridMonth"
      weekends={false}
      events={icalEvents}
    />
  );
}

export default LoadsheddingCalendar;
