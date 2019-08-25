import React from 'react';
import CssBaseline from '@material-ui/core/CssBaseline';
import Paper from '@material-ui/core/Paper';
import Grid from '@material-ui/core/Grid';
import TextField from '@material-ui/core/TextField';
import { withStyles } from '@material-ui/core/styles';
import MuiExpansionPanel from '@material-ui/core/ExpansionPanel';
import { navigation } from '../../../navigation'
import Card from '@material-ui/core/Card';
import CardActionArea from '@material-ui/core/CardActionArea';
import CardActions from '@material-ui/core/CardActions';
import CardContent from '@material-ui/core/CardContent';
import CardMedia from '@material-ui/core/CardMedia';
import { withRouter } from 'react-router-dom';
import { styles } from '../../shared/styles';
import { loadNodeSettings } from './actions'
import { connect } from 'react-redux';
import errorImage from '../../../../images/error.png'
import MuiExpansionPanelSummary from '@material-ui/core/ExpansionPanelSummary';
import MuiExpansionPanelDetails from '@material-ui/core/ExpansionPanelDetails';
import Typography from '@material-ui/core/Typography';
import CircularProgress from '@material-ui/core/CircularProgress';

const ExpansionPanel = withStyles({
  root: {
    border: '1px solid rgba(0,0,0,.125)',
    boxShadow: 'none',
    '&:not(:last-child)': {
      borderBottom: 0,
    },
    '&:before': {
      display: 'none',
    },
  },
  expanded: {
    margin: 'auto',
  },
})(MuiExpansionPanel);

const ExpansionPanelSummary = withStyles({
  root: {
    backgroundColor: 'rgba(0,0,0,.03)',
    borderBottom: '1px solid rgba(0,0,0,.125)',
    marginBottom: -1,
    minHeight: 56,
    '&$expanded': {
      minHeight: 56,
    },
  },
  content: {
    '&$expanded': {
      margin: '12px 0',
    },
  },
  expanded: {},
})(props => <MuiExpansionPanelSummary {...props} />);

ExpansionPanelSummary.muiName = 'ExpansionPanelSummary';

const ExpansionPanelDetails = withStyles(theme => ({
  root: {
    padding: theme.spacing.unit * 2,
  },
}))(MuiExpansionPanelDetails);

class ChannelSettings extends React.Component {

  state = {
    isLoading: false,
    error: '',
    data: '',
    isSuccess: false,
  };

  componentDidMount() {
    this.props.titleHandler("ChannelSettings")
    this.props.loadNodeSettings(this.props.history.location.state.host)
  }

  state = {
    expanded: 'panel1',
  };

  handleChange = panel => (event, expanded) => {
    this.setState({
      expanded: expanded ? panel : false,
    });
  };

  render() {
    const { expanded } = this.state;
    const { classes } = this.props;

    if (this.props.error) {
      return (<Card className={classes.errorCard}>
        <CardActionArea>
          <CardContent>
            <img src={errorImage} className={classes.errorImage} alt='error icon' />
            <Typography gutterBottom variant="h5" component="h2">
              Error while loading channels</Typography>
            <Typography component="p">
              {this.props.error}</Typography>
          </CardContent>
        </CardActionArea>
      </Card>)
    }

    let data = this.props.data.data;

    if (data)
      return (
        <div>
          <ExpansionPanel expanded={expanded === 'panel1'} onChange={this.handleChange('panel1')}>
            <ExpansionPanelSummary>
              <Typography>Block 0 </Typography>
            </ExpansionPanelSummary>
            <ExpansionPanelDetails>
              <Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.block0Hash}
                </Typography>
                <Typography component="p">
                  block0Hash
                </Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.block0Time}
                </Typography>
                <Typography component="p">
                  block0Time
                 </Typography>
              </Typography>
            </ExpansionPanelDetails>
          </ExpansionPanel>
          <ExpansionPanel expanded={expanded === 'panel2'} onChange={this.handleChange('panel2')}>
            <ExpansionPanelSummary>
              <Typography>Fee</Typography>
            </ExpansionPanelSummary>
            <ExpansionPanelDetails>
              <Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.fees.certificate}
                </Typography>
                <Typography component="p">
                  certificate
                 </Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.fees.coefficient}
                </Typography>
                <Typography component="p">
                  coefficient
                 </Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.fees.constant}
                </Typography>
                <Typography component="p">
                  constant
                 </Typography>
              </Typography>
            </ExpansionPanelDetails>
          </ExpansionPanel>
          <ExpansionPanel expanded={expanded === 'panel3'} onChange={this.handleChange('panel3')}>
            <ExpansionPanelSummary>
              <Typography>Other</Typography>
            </ExpansionPanelSummary>
            <ExpansionPanelDetails>
              <Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.maxTxsPerBlock} </Typography>
                <Typography component="p">
                  maxTxsPerBlock
                 </Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.consensusVersion} </Typography>
                <Typography component="p">
                  consensusVersion
                 </Typography>
                <Typography gutterBottom variant="h5" component="h2">
                  {data.currSlotStartTime} </Typography>
                <Typography component="p">
                  currSlotStartTime
                 </Typography>

              </Typography>
            </ExpansionPanelDetails>
          </ExpansionPanel>
        </div>
      );

    return (<CircularProgress className={classes.progress} size={100} />)
  }
}

const mapStateToProps = state => {
  console.log("map")
  console.log(state)
  return {
    error: state.settings.error,
    isLoading: state.settings.isLoading,
    data: state.settings.data,
    host: state.settings
  }
}

export default connect(mapStateToProps, { loadNodeSettings, })(withStyles(styles)(withRouter(ChannelSettings)));